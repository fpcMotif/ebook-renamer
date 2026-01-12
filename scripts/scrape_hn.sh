#!/bin/bash
# Hacker News Scraper using agent-browser
# Usage: ./scrape_hn.sh [output_file] [num_pages]

set -e

OUTPUT_FILE="${1:-hn_stories.json}"
NUM_PAGES="${2:-1}"
TEMP_DIR=$(mktemp -d)

cleanup() {
    agent-browser close 2>/dev/null || true
    rm -rf "$TEMP_DIR"
}
trap cleanup EXIT

echo "Scraping Hacker News ($NUM_PAGES page(s))..."

ALL_STORIES="[]"

for ((page=1; page<=NUM_PAGES; page++)); do
    if [ "$page" -eq 1 ]; then
        URL="https://news.ycombinator.com"
    else
        URL="https://news.ycombinator.com/news?p=$page"
    fi

    echo "  Page $page: $URL"

    agent-browser open "$URL" >/dev/null 2>&1
    agent-browser wait --load networkidle >/dev/null 2>&1

    # Extract stories from current page
    PAGE_STORIES=$(agent-browser eval "
const stories = [];
const rows = document.querySelectorAll('.athing');
rows.forEach((row, i) => {
  const titleEl = row.querySelector('.titleline > a');
  const siteEl = row.querySelector('.sitestr');
  const subtext = row.nextElementSibling;
  const scoreEl = subtext?.querySelector('.score');
  const commentsEl = [...(subtext?.querySelectorAll('a') || [])].find(a => a.textContent.includes('comment'));
  const ageEl = subtext?.querySelector('.age a');
  const hnId = row.id;

  if (titleEl) {
    stories.push({
      id: hnId,
      title: titleEl.textContent,
      url: titleEl.href,
      site: siteEl?.textContent || 'news.ycombinator.com',
      points: parseInt(scoreEl?.textContent?.replace(' points', '') || '0'),
      comments: parseInt(commentsEl?.textContent?.replace(/[^0-9]/g, '') || '0'),
      age: ageEl?.textContent || '',
      hn_url: 'https://news.ycombinator.com/item?id=' + hnId
    });
  }
});
JSON.stringify(stories);
" --json 2>/dev/null | jq -r '.data.result' 2>/dev/null || echo "[]")

    # Merge with existing stories
    ALL_STORIES=$(echo "$ALL_STORIES" "$PAGE_STORIES" | jq -s 'add')

    # Small delay between pages to be polite
    [ "$page" -lt "$NUM_PAGES" ] && sleep 1
done

# Create final output with metadata
TOTAL=$(echo "$ALL_STORIES" | jq 'length')
echo "$ALL_STORIES" | jq --arg ts "$(date -u +%Y-%m-%dT%H:%M:%SZ)" --arg total "$TOTAL" '{
  scraped_at: $ts,
  total: ($total | tonumber),
  stories: .
}' > "$OUTPUT_FILE"

agent-browser close >/dev/null 2>&1

echo "Done! Scraped $TOTAL stories -> $OUTPUT_FILE"

# Show top 5 by points
echo ""
echo "Top 5 by points:"
jq -r '.stories | sort_by(-.points) | .[0:5] | .[] | "  \(.points) pts - \(.title[0:60])..."' "$OUTPUT_FILE"
