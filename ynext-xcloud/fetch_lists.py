import requests
import re

url = "https://www.xbox.com/pt-BR/play"
headers = {
    "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64)"
}
res = requests.get(url, headers=headers)
print("Status:", res.status_code)

# Procurar padrões de GUID na página que possam ser SIGL IDs
import json

# Tentar achar a configuração de listas
# Em geral o xbox.com/play baixa um config.json
