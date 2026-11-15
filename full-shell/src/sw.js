var CACHE = 'yinji-v8';
var ASSETS = ['./', './index.html', './manifest.webmanifest', './brand.html', './icon.svg', './icon-192.png', './icon-512.png', './icon-maskable.svg', './favicon-16.png', './favicon-32.png', './apple-touch-icon.png'];

self.addEventListener('install', function (e) {
  self.skipWaiting();
  e.waitUntil(caches.open(CACHE).then(function (c) {
    return c.addAll(ASSETS);
  }).catch(function () {}));
});

self.addEventListener('activate', function (e) {
  e.waitUntil(caches.keys().then(function (ks) {
    return Promise.all(ks.map(function (k) {
      if (k !== CACHE) return caches.delete(k);
    }));
  }).then(function () { return self.clients.claim(); }));
});

self.addEventListener('fetch', function (e) {
  if (e.request.method !== 'GET') return;
  e.respondWith(
    fetch(e.request).then(function (r) {
      var copy = r.clone();
      caches.open(CACHE).then(function (c) { c.put(e.request, copy); }).catch(function () {});
      return r;
    }).catch(function () {
      return caches.match(e.request).then(function (hit) {
        return hit || new Response('离线中，请用本地文件打开印迹', {
          status: 503, headers: { 'Content-Type': 'text/plain; charset=utf-8' }
        });
      });
    })
  );
});
