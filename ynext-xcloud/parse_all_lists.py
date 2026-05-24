import json, re

with open('/tmp/xbox_cloud.html', 'r') as f:
    content = f.read()

match = re.search(r'window\.__INITIAL_STATE__\s*=\s*({.*?});', content)
if match:
    state = json.loads(match.group(1))
    
    # Try to find catalog items
    try:
        pages = state['catalog']['pages']
        for page_id, page_data in pages.items():
            if 'lists' in page_data:
                for lst in page_data['lists']:
                    print(f"List: {lst.get('title', 'NO TITLE')} | ID: {lst.get('id', 'NO ID')}")
    except KeyError:
        print("Could not find catalog pages")
