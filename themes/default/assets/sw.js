// Service Worker for offline caching
const CACHE_NAME = 'dove-site-v2';
// Derive site root from SW location to support base_path
const ROOT = new URL('../', self.location).pathname; // e.g. '/base_path/'
const p = (rel) => ROOT + String(rel).replace(/^\/+/,'');
const OFFLINE_URL = p('assets/offline.html');
const urlsToCache = [
  p(''),
  p('intranet/'),
  p('assets/styles.css'),
  p('assets/app.js'),
  p('assets/qrcode.min.js'),
  p('assets/offline.html'),
  p('assets/favicon.svg'),
  p('assets/favicon.png'),
  p('assets/favicon-f.svg')
];

// Install event - cache static assets
self.addEventListener('install', event => {
  event.waitUntil(
    caches.open(CACHE_NAME)
      .then(cache => {
        console.log('Opened cache');
        return cache.addAll(urlsToCache);
      })
      .then(() => self.skipWaiting())
  );
});

// Fetch event - serve cached content when offline
self.addEventListener('fetch', event => {
  event.respondWith(
    caches.match(event.request)
      .then(response => {
        // Return cached version if available
        if (response) {
          return response;
        }
        
        // Clone the request because it's a stream and can only be consumed once
        const fetchRequest = event.request.clone();
        
        // Try to fetch from network
        return fetch(fetchRequest)
          .then(response => {
            // Check if we received a valid response
            if (!response || response.status !== 200 || response.type !== 'basic') {
              return response;
            }
            
            // Clone the response because it's a stream and can only be consumed once
            const responseToCache = response.clone();
            
            // Cache the response for future offline use
            caches.open(CACHE_NAME)
              .then(cache => {
                cache.put(event.request, responseToCache);
              });
              
            return response;
          })
          .catch(() => {
            // If fetch fails (offline), return an offline fallback page
            return caches.match(OFFLINE_URL);
          });
      })
    );
});

// Activate event - clean up old caches
self.addEventListener('activate', event => {
  const cacheWhitelist = [CACHE_NAME];
  
  event.waitUntil(
    caches.keys()
      .then(cacheNames => {
        return Promise.all(
          cacheNames.map(cacheName => {
            if (cacheWhitelist.indexOf(cacheName) === -1) {
              return caches.delete(cacheName);
            }
          })
        );
      })
      .then(() => self.clients.claim())
  );
});
