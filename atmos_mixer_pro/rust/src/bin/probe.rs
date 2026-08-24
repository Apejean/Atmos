use symphonia::core::probe::Hint;
use symphonia::core::formats::FormatOptions;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::io::MediaSourceStream;
use std::fs::File;

fn main() {
    let path = "/Users/Allweno/Downloads/591775__cabled_mess__surround-test_51_l-r-c-lfe-ls-rs (1).wav";
    let file = Box::new(File::open(path).unwrap());
    let mss = MediaSourceStream::new(file, Default::default());
    let mut hint = Hint::new();
    hint.with_extension("wav");
    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .unwrap();
    let track = probed.format.default_track().unwrap();
    println!("channels mask: {:?}", track.codec_params.channels);
}
