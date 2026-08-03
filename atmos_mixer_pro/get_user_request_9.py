import json

with open("/Users/Allweno/.gemini/antigravity/brain/a3ded415-d30c-416d-9b4f-3b9f8124c618/.system_generated/logs/transcript_full.jsonl", "r") as f:
    for line in f:
        try:
            data = json.loads(line)
            if data.get("type") == "USER_INPUT" and "잠재적 버그 및 메모리 누수" in data.get("content", ""):
                print(data["content"])
        except:
            pass
