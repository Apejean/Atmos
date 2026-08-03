import json

with open("/Users/Allweno/.gemini/antigravity/brain/a3ded415-d30c-416d-9b4f-3b9f8124c618/.system_generated/logs/transcript_full.jsonl", "r") as f:
    for line in f:
        if "무제한 메모리 누수" in line:
            data = json.loads(line)
            if "content" in data:
                print(data["content"])
