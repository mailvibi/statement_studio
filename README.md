# Statement Studio WASM

Static GitHub Pages port of the Python Statement Studio pipeline. It processes statement CSVs and the category mapping entirely in the browser, then creates a downloadable HTML report.

## Local build

```bash
cargo test
cargo build --release --target wasm32-unknown-unknown
mkdir -p _site
cp index.html styles.css app.js shopname_category_mapping.json _site/
cp target/wasm32-unknown-unknown/release/statement_studio_wasm.wasm _site/
python3 -m http.server 4173 --directory _site
```

Open `http://127.0.0.1:4173/`. The GitHub Actions workflow publishes the same static bundle to GitHub Pages on pushes to `main`.