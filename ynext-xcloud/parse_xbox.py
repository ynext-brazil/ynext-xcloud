import json
import re

with open("/tmp/xbox_cloud.html", "r", encoding="utf-8") as f:
    html = f.read()

# Look for patterns like {"id":"uuid","title":"text",...}
pattern = r'\{"id":"([a-f0-9\-]{36})","title":"([^"]+)"'
for match in re.finditer(pattern, html):
    print(f"ID: {match.group(1)} | Title: {match.group(2)}")
