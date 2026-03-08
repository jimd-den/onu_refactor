/**
 * Ọ̀nụ Offline Playground — app.js
 *
 * Phase 3: CodeMirror 6 editor with live linting via OnuCompiler.lint()
 * Phase 4: PWA offline execution via OnuCompiler.compile() + WebAssembly.instantiate()
 *
 * Architecture:
 *  - The editor is CodeMirror 6 (mobile-friendly, touch-capable).
 *  - The compiler is the ~1 MB `onu_compiler_bg.wasm` Rust module, loaded once
 *    and cached by the Service Worker so it works in Airplane Mode.
 *  - Every keystroke calls OnuCompiler.lint() for red-squiggle diagnostics.
 *  - Pressing "Run" calls OnuCompiler.compile() to get raw .wasm bytes, then
 *    instantiates them in the browser's native WebAssembly JIT.
 */

import { EditorState }       from "@codemirror/state";
import { EditorView, keymap, lineNumbers, highlightActiveLineGutter,
         highlightActiveLine, drawSelection }   from "@codemirror/view";
import { defaultKeymap, historyKeymap, history } from "@codemirror/commands";
import { syntaxHighlighting, defaultHighlightStyle } from "@codemirror/language";
import { linter, lintGutter }                   from "@codemirror/lint";

// ── DOM helpers ───────────────────────────────────────────────────────────────

const $statusDot   = document.getElementById("status-dot");
const $statusLabel = document.getElementById("status-label");
const $runBtn      = document.getElementById("run-btn");
const $clearBtn    = document.getElementById("clear-btn");
const $output      = document.getElementById("output");
const $offlineBadge = document.getElementById("offline-badge");

function setStatus(state, label) {
  $statusDot.className = `status-dot ${state}`;
  $statusLabel.textContent = label;
}

function appendOutput(text, cls = "out-info") {
  const span = document.createElement("span");
  span.className = cls;
  span.textContent = text;
  if ($output.firstChild?.classList?.contains("out-muted")) {
    $output.innerHTML = "";
  }
  $output.appendChild(span);
  $output.scrollTop = $output.scrollHeight;
}

// ── Offline detection ─────────────────────────────────────────────────────────

function updateOnlineStatus() {
  $offlineBadge.style.display = navigator.onLine ? "none" : "inline-block";
}
window.addEventListener("online",  updateOnlineStatus);
window.addEventListener("offline", updateOnlineStatus);
updateOnlineStatus();

// ── Service Worker registration ───────────────────────────────────────────────

if ("serviceWorker" in navigator) {
  navigator.serviceWorker
    .register("service-worker.js")
    .catch(err => console.warn("SW registration failed:", err));
}

// ── WASM compiler bootstrap ───────────────────────────────────────────────────

let OnuCompiler = null; // will be set once the WASM module loads

async function loadCompiler() {
  setStatus("loading", "Loading compiler…");
  try {
    // wasm-bindgen generates two files:
    //   onu_compiler.js  — the JS glue
    //   onu_compiler_bg.wasm — the ~1 MB binary
    const { default: init, OnuCompiler: Compiler } =
      await import("./onu_compiler.js");
    await init("./onu_compiler_bg.wasm");
    OnuCompiler = Compiler;
    setStatus("ready", "Compiler ready");
    $runBtn.disabled = false;
  } catch (err) {
    setStatus("error", "Compiler failed to load");
    appendOutput(`Compiler load error: ${err.message}\n`, "out-error");
    console.error(err);
  }
}

// ── CodeMirror 6 linter ───────────────────────────────────────────────────────

/** Calls OnuCompiler.lint() and converts results to CM6 Diagnostic objects. */
function onuLinter(view) {
  if (!OnuCompiler) return [];
  const source = view.state.doc.toString();
  try {
    const raw = OnuCompiler.lint(source);
    const diags = JSON.parse(raw);
    return diags.map(d => {
      // Convert (line, col) → CM6 character offsets
      const lineObj = view.state.doc.line(Math.max(1, (d.line || 0) + 1));
      const from = lineObj.from + Math.max(0, d.col || 0);
      return {
        from,
        to: Math.min(from + 1, lineObj.to),
        severity: "error",
        message: d.message,
      };
    });
  } catch {
    return [];
  }
}

// ── CodeMirror 6 setup ────────────────────────────────────────────────────────

const defaultSource = `the module called HelloPlayground
    with concern: first discourse in the browser

the effect behavior called run
    with intent: greet the world
    takes: nothing
    delivers: nothing
    as:
        broadcasts "Hello from Ọ̀nụ in the browser!"
`;

const editorView = new EditorView({
  state: EditorState.create({
    doc: defaultSource,
    extensions: [
      history(),
      lineNumbers(),
      highlightActiveLineGutter(),
      highlightActiveLine(),
      drawSelection(),
      syntaxHighlighting(defaultHighlightStyle),
      keymap.of([...defaultKeymap, ...historyKeymap]),
      lintGutter(),
      linter(onuLinter, { delay: 300 }),
      EditorView.theme({
        "&": { height: "100%" },
        ".cm-scroller": { overflow: "auto" },
      }),
    ],
  }),
  parent: document.getElementById("editor-container"),
});

// ── Run button ────────────────────────────────────────────────────────────────

$runBtn.addEventListener("click", async () => {
  if (!OnuCompiler) return;

  $runBtn.disabled = true;
  $output.innerHTML = "";
  const source = editorView.state.doc.toString();

  appendOutput("Compiling…\n", "out-muted");

  try {
    // 1. Compile source → raw .wasm bytecode
    const wasmBytes = OnuCompiler.compile(source);

    // 2. Define the I/O boundary (replaces raw x86 syscalls)
    const output = [];
    const importObject = {
      env: {
        /**
         * onu_write(ptr: i32, len: i32) — writes `len` bytes from WASM
         * linear memory starting at `ptr` to the HTML terminal.
         */
        onu_write: (ptr, len) => {
          const mem = new Uint8Array(instance.exports.memory.buffer);
          const bytes = mem.slice(ptr, ptr + len);
          const text = new TextDecoder().decode(bytes);
          output.push(text);
        },
        /**
         * onu_print_i64(val: BigInt) — formats an i64 integer for output.
         */
        onu_print_i64: (val) => {
          output.push(String(val));
        },
      },
    };

    // 3. Instantiate and run the compiled program
    const { instance } = await WebAssembly.instantiate(wasmBytes, importObject);
    instance.exports.run();

    // Flush any buffered output
    const text = output.join("");
    if (text) {
      appendOutput(text + (text.endsWith("\n") ? "" : "\n"), "out-success");
    } else {
      appendOutput("(program completed with no output)\n", "out-muted");
    }
  } catch (err) {
    appendOutput(`\nError: ${err.message}\n`, "out-error");
    console.error(err);
  } finally {
    $runBtn.disabled = false;
  }
});

$clearBtn.addEventListener("click", () => {
  $output.innerHTML = '<span class="out-muted">— output will appear here —</span>';
});

// ── Boot ──────────────────────────────────────────────────────────────────────

loadCompiler();
