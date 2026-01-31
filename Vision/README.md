# Viewer for uRing Crawler

Viewer for Yonsei University notices with a refreshed UX layer and pluggable data sources.

## Data Sources

The viewer can load notices from multiple sources (priority order):

1. `NEXT_PUBLIC_NOTICES_URL` (full URL to a JSON feed)
2. `NEXT_PUBLIC_S3_BASE_URL` + `NEXT_PUBLIC_S3_PREFIX` (campus-aware S3 New/ data)
3. `/public/data/notices.json` (local snapshot)

### S3 Environment Variables

- `NEXT_PUBLIC_S3_BASE_URL`: e.g. `https://<bucket>.s3.amazonaws.com`
- `NEXT_PUBLIC_S3_PREFIX`: defaults to `uRing`
- `NEXT_PUBLIC_S3_SITEMAP_KEY`: defaults to `uRing/config/sitemap.json`

Make sure the S3 bucket (or CloudFront) allows CORS for `GET` requests from the viewer origin.

### Optional Overrides

- `NEXT_PUBLIC_NOTICES_BASE_URL`: base URL used to fetch `/data/notices.json`
- `NEXT_PUBLIC_NOTICES_URL`: full URL to a notices JSON feed
