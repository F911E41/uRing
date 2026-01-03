# Mapper/get_useful_boards.py

from bs4 import BeautifulSoup
from urllib.parse import urljoin, urlparse

import re
import json
import requests

# Mapping of keywords to board IDs and names
KEYWORD_MAP = {
    '학부공지': {'id': 'academic', 'name': '학사공지'},
    '대학원공지': {'id': 'grad_notice', 'name': '대학원공지'},
    '장학': {'id': 'scholarship', 'name': '장학공지'},
    '취업': {'id': 'career', 'name': '취업/진로'},
    '공지사항': {'id': 'notice', 'name': '일반공지'},
    '학사공지': {'id': 'academic', 'name': '학사공지'}
}

# For departments needing manual review
manual_review_needed = []

# Is the link a valid board link?
def is_valid_board_link(text, href):
    # If the link is likely not a board link, return False
    blacklist = ['articleNo', 'article_no', 'mode=view', 'seq', 'view.do', 'board_seq']
    if any(word in href for word in blacklist):
        return False
    
    # If the text is too long, it's likely a notice title rather than a board name
    if len(text) > 20:
        return False
        
    # Handle icon links like 'more' or '+' (optional)
    # if text == '+' or 'more' in text.lower(): return True

    return True

# Detect CMS type and return appropriate selectors
def detect_cms_and_get_selectors(soup, url):
    html_str = str(soup).lower()
    
    # Standard Yonsei CMS
    if ".do" in url or "c-board-title" in html_str:
        return {
            "row_selector": "tr:has(a.c-board-title)",
            "title_selector": "a.c-board-title",
            "date_selector": "td:nth-last-child(1)",
            "attr_name": "href"
        }
    
    # Detect by presence of XE/Rhymix specific classes or URL patterns
    if "xe" in html_str or "rhymix" in html_str or "mid=" in url:
        return {
            "row_selector": "li.xe-list-board-list--item:not(.xe-list-board-list--header)",
            "title_selector": "a.xe-list-board-list__title-link",
            "date_selector": ".xe-list-board-list__created_at",
            "attr_name": "href"
        }

    # Fallback
    return None

# Discover useful boards from a department URL
def discover_boards(dept_info, dept_url):
    boards = []
    if not dept_url: return boards
    
    headers = {
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
    }

    try:
        res = requests.get(dept_url, headers=headers, timeout=7)
        soup = BeautifulSoup(res.text, 'html.parser')
        
        cms_selectors = detect_cms_and_get_selectors(soup, dept_url)
        
        # If the `url` is `NOT_FOUND`, we cannot proceed
        if dept_url == "NOT_FOUND":
            manual_review_needed.append({
                "campus": dept_info['campus'],
                "name": dept_info['name'],
                "url": dept_url,
                "reason": "Homepage URL is `NOT_FOUND`"
            })
            return []

        # If CMS detection fails, add to manual review list
        if cms_selectors is None:
            manual_review_needed.append({
                "campus": dept_info['campus'],
                "name": dept_info['name'],
                "url": dept_url,
                "reason": "Unknown CMS Structure"
            })
            return []

        links = soup.find_all('a', href=True)
        seen_urls = set()
        id_counts = {} # To handle duplicate IDs
        dept_domain = urlparse(dept_url).netloc.lower()

        for link in links:
            text = link.get_text(strip=True)
            href = link['href']
            if not is_valid_board_link(text, href): continue

            full_url = urljoin(dept_url, href)
            if full_url in seen_urls or 'javascript' in href or '#' in href: continue
            
            # Ignore if subdomain is different
            link_domain = urlparse(full_url).netloc.lower()
            if link_domain != dept_domain: continue

            for key, meta in KEYWORD_MAP.items():
                if key in text or (re.search(r'notice|scholar|academic', href.lower()) and key in text):
                    base_id = meta['id']
                    id_counts[base_id] = id_counts.get(base_id, 0) + 1
                    final_id = f"{base_id}_{id_counts[base_id]}" if id_counts[base_id] > 1 else base_id

                    boards.append({
                        "id": final_id,
                        "name": text if text else meta['name'],
                        "url": full_url,
                        **cms_selectors
                    })
                    seen_urls.add(full_url)
                    break
                    
    except Exception as e:
        manual_review_needed.append({
            "campus": dept_info['campus'],
            "name": dept_info['name'],
            "url": dept_url,
            "reason": f"Connection Error: {str(e)}"
        })
        
    return boards

# Load existing Yonsei departments data
with open('result/yonsei_departments.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

# Iterate through each department to find useful boards
for campus in data:
    for college in campus['colleges']:
        for dept in college['departments']:
            # To pass campus info for manual review logging
            info = {"campus": campus['campus'], "name": dept['name']}
            print(f"Searching boards for: [{campus['campus']}] {dept['name']} ({dept['url']})")
            dept['boards'] = discover_boards(info, dept['url'])

# Save updated data with boards
with open('result/yonsei_departments_boards.json', 'w', encoding='utf-8') as f:
    json.dump(data, f, ensure_ascii=False, indent=4)

# Save manual review needed departments
with open('result/manual_review_needed.json', 'w', encoding='utf-8') as f:
    json.dump(manual_review_needed, f, ensure_ascii=False, indent=4)

print("\nUpdated 'result/yonsei_departments_boards.json' with discovered boards successfully.")
print(f"Saved departments needing manual review for {len(manual_review_needed)} to 'result/manual_review_needed.json'.")
