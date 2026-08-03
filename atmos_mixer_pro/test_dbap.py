import json

with open("/Users/Allweno/.gemini/antigravity/brain/a3ded415-d30c-416d-9b4f-3b9f8124c618/.system_generated/logs/transcript_full.jsonl", "r") as f:
    for line in f:
        try:
            data = json.loads(line)
            if data.get("source") == "SYSTEM" and "DBAP" in data.get("content", ""):
                print(data["content"])
        except:
            pass
