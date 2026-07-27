import urllib.request
import json
import re

url = "https://api.github.com/repos/pierreaubert/spinorama/git/trees/master?recursive=1"
req = urllib.request.Request(url, headers={'User-Agent': 'Mozilla/5.0'})
try:
    with urllib.request.urlopen(req) as response:
        data = json.loads(response.read().decode())
    
    speakers = []
    for item in data.get("tree", []):
        path = item.get("path", "")
        # Match datas/eq/{speaker_name}/iir-autoeq.txt
        match = re.match(r"^datas/eq/([^/]+)/iir-autoeq\.txt$", path)
        if match:
            speakers.append(match.group(1))
            
    with open("src/lib/spinorama_index.json", "w", encoding="utf-8") as f:
        json.dump(speakers, f, ensure_ascii=False)
        
    print(f"Saved {len(speakers)} speakers to spinorama_index.json")
except Exception as e:
    print(f"Error: {e}")
