// Keeps the cache of the version before, which tabs opened before the update still load from.
self.addEventListener('install', (e) => e.waitUntil(caches.open(CACHE).then((c) => c.addAll(FILES)).then(() => self.skipWaiting())))

self.addEventListener('activate', (e) =>
  e.waitUntil(
    caches
      .keys()
      .then((keys) => Promise.all(keys.filter((k) => k !== CACHE).slice(0, -1).map((k) => caches.delete(k))))
      .then(() => self.clients.claim()),
  ),
)

self.addEventListener('fetch', (e) => {
  if (e.request.method === 'GET') e.respondWith(caches.match(e.request, { ignoreSearch: true, ignoreVary: true }).then((r) => r ?? fetch(e.request)))
})
