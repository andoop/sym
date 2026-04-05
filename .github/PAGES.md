# Enable GitHub Pages for this site

Static files live in **`docs/`**: `index.html`, `assets/site.css`, `README.zh.html`, plus `.nojekyll` (disables Jekyll so paths behave predictably).

## Steps

1. Open **https://github.com/andoop/sym/settings/pages**
2. Under **Build and deployment** → **Source**: **Deploy from a branch**
3. **Branch**: `main` · **Folder**: **`/docs`** · Save
4. After the workflow runs (~1–2 minutes), the site is at **`https://andoop.github.io/sym/`**

## Custom domain (optional)

Add your domain in the same Pages settings and follow GitHub’s DNS instructions. Remove or update any hard-coded URLs if you change the hostname.

## Local preview

From repo root:

```bash
cd docs && python3 -m http.server 8080
# open http://127.0.0.1:8080/
```
