import json, re

with open('/tmp/xbox_cloud.html', 'r') as f:
    content = f.read()

match = re.search(r'window\.__INITIAL_STATE__\s*=\s*({.*?});', content)
if match:
    state = json.loads(match.group(1))
    print(state.keys())
    if 'catalogs' in state:
        print("catalogs", state['catalogs'].keys())
    if 'gameCatalog' in state:
        print("gameCatalog keys:", state['gameCatalog'].keys())
