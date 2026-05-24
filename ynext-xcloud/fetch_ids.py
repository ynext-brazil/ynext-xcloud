import urllib.request
import re

url = "https://www.xbox.com/pt-BR/play"
req = urllib.request.Request(url, headers={'User-Agent': 'Mozilla/5.0'})
try:
    html = urllib.request.urlopen(req).read().decode('utf-8')
    matches = re.findall(r'"id":"([0-9a-f-]{36})","title":"([^"]+)"', html)
    for m in set(matches):
        print(f"{m[1]}: {m[0]}")
except Exception as e:
    print(e)
