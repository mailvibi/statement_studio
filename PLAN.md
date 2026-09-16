# Statement Studio WASM Port

## Plan

1. **Define the browser contract** - Complete
   - Use multiple CSV file inputs for statements and one JSON mapping file.
   - Process files locally in the browser; never upload statement data.
   - Generate a downloadable HTML report and show summary metrics in the app.

2. **Build the WASM processing core** - Complete
   - Implement CSV parsing, statement combination, column removal, merchant categorization, outgoing-transaction filtering, and category aggregation in Rust.
   - Expose one JSON-in/JSON-out function to the browser so the UI remains simple and testable.
   - Add Rust unit tests for CSV quoting, category matching, negative amount handling, and aggregation order.

3. **Create the GitHub Pages frontend** - Complete
   - Add a focused Statement Studio interface with file pickers, mapping-file selection, activity log, summary cards, and report download.
   - Include the default mapping as a bundled static asset and allow users to replace it.
   - Load the generated WASM module from relative paths so project pages work under a GitHub Pages subpath.

4. **Add static hosting/build configuration** - Complete
   - Add Cargo and frontend build metadata plus a GitHub Actions workflow that builds the WASM package and publishes the `dist` directory.
   - Keep the output self-contained and suitable for GitHub Pages without a backend.

5. **Validate end to end** - Complete
   - Run Rust tests and the release WASM build.
   - Run a browser smoke test against the built static site with representative CSV data.
   - Check the generated report and confirm the repository contains only deployable source/build files.