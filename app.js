const statementInput = document.querySelector("#statements");
const mappingInput = document.querySelector("#mapping");
const runButton = document.querySelector("#run");
const downloadButton = document.querySelector("#download");
const statusPill = document.querySelector("#status-pill");
const state = { mapping: null, result: null, wasm: null, report: null };

const money = (value) => `${value.toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 })} EUR`;
const escapeHtml = (value) => String(value).replace(/[&<>"']/g, (character) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#039;" }[character]));

function setStatus(label, kind = "") {
  statusPill.textContent = label;
  statusPill.className = `status-pill ${kind}`;
}

async function loadWasm() {
  const response = await fetch("statement_studio_wasm.wasm");
  const bytes = await response.arrayBuffer();
  const { instance } = await WebAssembly.instantiate(bytes, {});
  state.wasm = instance.exports;
}

async function loadDefaultMapping() {
  const response = await fetch("shopname_category_mapping.json");
  if (!response.ok) throw new Error("The default category mapping could not be loaded.");
  state.mapping = await response.json();
}

function encodePayload(mapping, files) {
  const mappingText = Object.entries(mapping).flatMap(([category, items]) => items.map((item) => `${category}\t${item}`)).join("\n") + "\n";
  const sections = [String(new TextEncoder().encode(mappingText).length), "\n", mappingText, String(files.length), "\n"];
  for (const file of files) {
    const name = file.name;
    sections.push(String(new TextEncoder().encode(name).length), "\n", name);
    sections.push(String(new TextEncoder().encode(file.content).length), "\n", file.content);
  }
  return new TextEncoder().encode(sections.join(""));
}

function executeWasm(payload) {
  const pointer = state.wasm.alloc(payload.length);
  new Uint8Array(state.wasm.memory.buffer, pointer, payload.length).set(payload);
  state.wasm.process_pipeline(pointer, payload.length);
  const resultPointer = state.wasm.result_pointer();
  const resultLength = state.wasm.result_length();
  const resultBytes = new Uint8Array(state.wasm.memory.buffer, resultPointer, resultLength);
  const resultText = new TextDecoder().decode(resultBytes);
  return JSON.parse(resultText);
}

function renderResult(result) {
  document.querySelector("#files-read").textContent = result.files;
  document.querySelector("#outgoing-count").textContent = result.outgoingRows;
  document.querySelector("#grand-total").textContent = money(result.grandTotal);
  document.querySelector("#category-count").textContent = `${result.categories.length} categories`;
  document.querySelector("#category-list").innerHTML = result.categories.length
    ? result.categories.map((item) => `<div class="category-row"><strong>${escapeHtml(item.category)}</strong><span>${money(item.amount)}</span></div>`).join("")
    : '<div class="empty-state">No outgoing transactions were found.</div>';
}

function buildReport(result) {
  const rows = result.categories.map((item) => `<tr><td>${escapeHtml(item.category)}</td><td>${money(item.amount)}</td></tr>`).join("");
  return `<!doctype html><html lang="en"><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>Statement Studio Report</title><style>body{font:16px Arial,sans-serif;background:#f5f1e8;color:#183b3b;margin:0;padding:30px}.container{max-width:800px;margin:auto;background:#fffdf8;padding:30px}h1{font:600 42px Georgia,serif}table{width:100%;border-collapse:collapse}th,td{text-align:left;padding:12px;border-bottom:1px solid #e5ded1}th{color:#65726b;font-size:12px;text-transform:uppercase}td:last-child,th:last-child{text-align:right}.total{margin-top:24px;padding:18px;background:#dfece4;font-weight:bold}</style></head><body><main class="container"><h1>Statement Studio</h1><p>Outgoing category summary</p><table><thead><tr><th>Category</th><th>Total</th></tr></thead><tbody>${rows}</tbody></table><div class="total">Grand total: ${money(result.grandTotal)}</div><p>${result.files} files read · ${result.outgoingRows} outgoing entries</p></main></body></html>`;
}

statementInput.addEventListener("change", () => {
  const count = statementInput.files.length;
  document.querySelector("#statement-label").textContent = count ? `${count} CSV${count === 1 ? "" : "s"} selected` : "Drop statement CSVs here";
  runButton.disabled = !count;
});

mappingInput.addEventListener("change", async () => {
  const file = mappingInput.files[0];
  if (!file) return;
  try { state.mapping = JSON.parse(await file.text()); document.querySelector("#mapping-label").textContent = file.name; }
  catch (error) { setStatus("Invalid mapping", "error"); document.querySelector("#mapping-label").textContent = error.message; }
});

runButton.addEventListener("click", async () => {
  runButton.disabled = true; setStatus("Processing...");
  try {
    if (!state.wasm) await loadWasm();
    if (!state.mapping) await loadDefaultMapping();
    const files = await Promise.all([...statementInput.files].map(async (file) => ({ name: file.name, content: await file.text() })));
    const response = executeWasm(encodePayload(state.mapping, files));
    if (!response.ok) throw new Error(response.error);
    state.result = response.data; state.report = buildReport(state.result); renderResult(state.result);
    downloadButton.disabled = false; setStatus("Report ready", "ready");
  } catch (error) { setStatus("Processing failed", "error"); document.querySelector("#category-list").innerHTML = `<div class="empty-state">${escapeHtml(error.message)}</div>`; }
  runButton.disabled = false;
});

downloadButton.addEventListener("click", () => {
  const link = document.createElement("a");
  link.href = URL.createObjectURL(new Blob([state.report], { type: "text/html" }));
  link.download = "statement-studio-report.html"; link.click(); URL.revokeObjectURL(link.href);
});

loadDefaultMapping().catch((error) => setStatus("Mapping unavailable", "error"));