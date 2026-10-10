//! 프로젝트 파일을 다른 PC에서 열 때 트랙 오디오·도면 경로를 다시 연결한다(HANDOFF 4번).
//! 프로젝트 파일에는 저장한 PC의 절대 경로가 남는다. 그 경로에 파일이 없으면 찾을 폴더(먼저 프로젝트
//! 폴더)와 그 하위 폴더에서 같은 파일 이름을 찾는다. 경로 구분자는 /와 \를 모두 알아본다 — macOS에서
//! 만든 프로젝트를 Windows에서 열 때와 그 반대 경우.
use crate::common::config::AppConfig;
use std::cmp::Reverse;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use unicode_normalization::UnicodeNormalization;

/// 하위 폴더를 이 깊이까지만 내려간다(USB 루트처럼 큰 폴더를 끝없이 뒤지지 않게).
const MAX_DEPTH: usize = 16;

/// 이름 비교용 키: 유니코드 결합형(NFC)으로 맞춘 뒤 소문자. 맥에서 온 이름은 한글이 자소 분리형(NFD)이기도 해서
/// Windows에서 만든 결합형 이름과 글자는 같아도 바이트가 다르다(HANDOFF 남은 일 14).
fn match_key(name: &str) -> String {
    if name.is_ascii() {
        return name.to_lowercase();
    }
    name.nfc().collect::<String>().to_lowercase()
}

/// 경로의 마지막 이름. /와 \ 둘 다 구분자로 본다. 폴더 경로(구분자로 끝남)면 None.
pub fn file_name_of(path: &str) -> Option<&str> {
    path.rsplit(['/', '\\']).next().filter(|name| !name.is_empty())
}

/// 경로의 상위 폴더 이름들(가까운 것부터, 비교용 키).
fn parent_names(path: &str) -> Vec<String> {
    let mut parts: Vec<&str> = path.split(['/', '\\']).filter(|part| !part.is_empty()).collect();
    parts.pop();
    parts.iter().rev().map(|part| match_key(part)).collect()
}

/// 찾을 폴더 하나의 파일 목록(비교용 이름 → 경로들). 숨김 파일·폴더(macOS가 USB에 남기는 `._` 파일 포함)는
/// 뺀다. 폴더 링크는 따라가지 않는다(서로 가리키는 링크로 끝없이 돌지 않게).
struct FolderIndex {
    root: PathBuf,
    by_name: HashMap<String, Vec<PathBuf>>,
}

impl FolderIndex {
    fn build(root: &Path) -> Self {
        let mut by_name: HashMap<String, Vec<PathBuf>> = HashMap::new();
        let mut stack = vec![(root.to_path_buf(), 0usize)];
        while let Some((dir, depth)) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&dir) else { continue };
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.starts_with('.') {
                    continue;
                }
                let Ok(kind) = entry.file_type() else { continue };
                let path = entry.path();
                if kind.is_dir() {
                    if depth < MAX_DEPTH {
                        stack.push((path, depth + 1));
                    }
                } else if kind.is_file() || (kind.is_symlink() && path.is_file()) {
                    by_name.entry(match_key(&name)).or_default().push(path);
                }
            }
        }
        Self { root: root.to_path_buf(), by_name }
    }

    /// 같은 이름 후보 중 원래 경로와 상위 폴더 이름이 가까운 쪽부터 가장 많이 겹치는 것. 같으면 얕은 것,
    /// 그다음 경로 순으로 고른다(열 때마다 같은 결과).
    fn best(&self, original: &str, name: &str) -> Option<&PathBuf> {
        let wanted = parent_names(original);
        let candidates = self.by_name.get(name)?;
        candidates.iter().min_by_key(|candidate| {
            let theirs = parent_names(&candidate.to_string_lossy());
            let matched = wanted.iter().zip(theirs.iter()).take_while(|(a, b)| a == b).count();
            let depth = candidate.strip_prefix(&self.root).map(|r| r.components().count()).unwrap_or(usize::MAX);
            (Reverse(matched), depth, (*candidate).clone())
        })
    }
}

/// 찾을 폴더들(앞의 것 먼저). 파일 목록은 처음 필요할 때 만든다 — 모두 제자리에 있으면 폴더를 뒤지지 않는다.
pub struct MediaFinder {
    dirs: Vec<PathBuf>,
    indexes: Vec<Option<FolderIndex>>,
}

impl MediaFinder {
    pub fn new(search_dirs: &[PathBuf]) -> Self {
        Self { dirs: search_dirs.to_vec(), indexes: search_dirs.iter().map(|_| None).collect() }
    }

    /// 저장된 경로에 파일이 있으면 그 경로, 없으면 찾을 폴더에서 이름으로 찾은 경로. 못 찾으면 None.
    pub fn resolve(&mut self, original: &str) -> Option<PathBuf> {
        if !original.is_empty() && Path::new(original).is_file() {
            return Some(PathBuf::from(original));
        }
        let name = match_key(file_name_of(original)?);
        for (dir, index) in self.dirs.iter().zip(self.indexes.iter_mut()) {
            let index = index.get_or_insert_with(|| FolderIndex::build(dir));
            if let Some(found) = index.best(original, &name) {
                return Some(found.clone());
            }
        }
        None
    }
}

/// 다시 연결한 결과.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct RelinkReport {
    /// (원래 경로, 새 경로)
    pub relinked: Vec<(String, String)>,
    /// 끝까지 못 찾은 원래 경로(같은 경로는 한 번만).
    pub missing: Vec<String>,
}

/// 엔진 설정의 트랙 오디오 경로만 다시 연결한다. 다른 값은 건드리지 않는다. 못 찾은 트랙은 원래 경로를 둔다.
pub fn relink_tracks(config: &mut AppConfig, search_dirs: &[PathBuf]) -> RelinkReport {
    let mut finder = MediaFinder::new(search_dirs);
    let mut report = RelinkReport::default();
    for track in config.rooms.iter_mut().flat_map(|room| room.tracks.iter_mut()) {
        if track.file_path.is_empty() || Path::new(&track.file_path).is_file() {
            continue;
        }
        match finder.resolve(&track.file_path) {
            Some(found) => {
                let found = found.to_string_lossy().into_owned();
                let original = std::mem::replace(&mut track.file_path, found.clone());
                report.relinked.push((original, found));
            }
            None => {
                if !report.missing.contains(&track.file_path) {
                    report.missing.push(track.file_path.clone());
                }
            }
        }
    }
    report
}
