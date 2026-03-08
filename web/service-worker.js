/**
 * Ọ̀nụ Playground — Service Worker
 *
 * Phase 4: Offline PWA Architecture
 *
 * Install event:   Downloads and caches five files so the entire
 *                  playground works in Airplane Mode after the first visit.
 * Fetch event:     Serves from the local cache first ("cache-first" strategy).
 *                  Falls back to the network for anything not cached.
 *
 * Cache busting:   Bump CACHE_VERSION whenever any cached file changes.
 */

const CACHE_VERSION = "onu-v1";

/** Files to pre-cache on the first visit. */
const PRECACHE_ASSETS = [
  "/index.html",
  "/app.js",
  "/onu_compiler_bg.wasm",   // ~1 MB Rust/WASM compiler binary
  "/onu_compiler.js",        // wasm-bindgen JS glue
  "/manifest.json",
];

// ── Install: pre-cache all assets ────────────────────────────────────────────

self.addEventListener("install", event => {
  event.waitUntil(
    caches
      .open(CACHE_VERSION)
      .then(cache => cache.addAll(PRECACHE_ASSETS))
      .then(() => self.skipWaiting())   // activate immediately
  );
});

// ── Activate: remove stale caches ────────────────────────────────────────────

self.addEventListener("activate", event => {
  event.waitUntil(
    caches.keys().then(keys =>
      Promise.all(
        keys
          .filter(key => key !== CACHE_VERSION)
          .map(key => caches.delete(key))
      )
    ).then(() => self.clients.claim())  // take control of all tabs
  );
});

// ── Fetch: cache-first with network fallback ──────────────────────────────────

self.addEventListener("fetch", event => {
  // Only handle GET requests for the playground's own origin.
  if (event.request.method !== "GET") return;

  event.respondWith(
    caches.match(event.request).then(cached => {
      if (cached) return cached;

      // Not in cache — go to the network and opportunistically cache the result.
      return fetch(event.request).then(response => {
        if (response.ok) {
          const clone = response.clone();
          caches.open(CACHE_VERSION).then(cache => cache.put(event.request, clone));
        }
        return response;
      });
    })
  );
});
