// Service worker registration + "new version available" state.
//
// The generated `sw.js` never activates an update on its own: a new build is
// installed into its own cache and then waits. We surface that waiting worker
// to the UI; when the user clicks Reload we tell it to take over and reload
// once it controls the page, so the whole session switches builds at once.

type Listener = () => void;

let waitingWorker: Pick<ServiceWorker, 'postMessage'> | null = null;
let reloadRequested = false;
const listeners = new Set<Listener>();

function emit(): void {
  listeners.forEach((listener) => listener());
}

export function subscribeToUpdate(listener: Listener): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

export function isUpdateAvailable(): boolean {
  return waitingWorker !== null;
}

/** Records an installed-but-waiting worker. Exported for tests. */
export function notifyUpdateAvailable(worker: Pick<ServiceWorker, 'postMessage'> | null): void {
  waitingWorker = worker;
  emit();
}

/** Activates the waiting worker; the page reloads once it takes control. */
export function applyUpdate(): void {
  if (!waitingWorker) return;
  reloadRequested = true;
  waitingWorker.postMessage({ type: 'SKIP_WAITING' });
}

export function registerServiceWorker(scriptUrl = '/sw.js'): void {
  if (typeof navigator === 'undefined' || !('serviceWorker' in navigator)) return;
  const container = navigator.serviceWorker;

  let reloading = false;
  container.addEventListener('controllerchange', () => {
    // The first install also fires controllerchange (clients.claim); only
    // reload when the user asked for the update.
    if (!reloadRequested || reloading) return;
    reloading = true;
    window.location.reload();
  });

  const watch = (worker: ServiceWorker) => {
    worker.addEventListener('statechange', () => {
      // `installed` with an existing controller means an update is waiting.
      if (worker.state === 'installed' && container.controller) {
        notifyUpdateAvailable(worker);
      }
    });
  };

  container
    .register(scriptUrl)
    .then((registration) => {
      if (registration.waiting && container.controller) {
        notifyUpdateAvailable(registration.waiting);
      }
      if (registration.installing) watch(registration.installing);
      registration.addEventListener('updatefound', () => {
        if (registration.installing) watch(registration.installing);
      });
      // Long-lived tabs: look for a new deploy whenever the tab is refocused.
      document.addEventListener('visibilitychange', () => {
        if (document.visibilityState === 'visible') {
          registration.update().catch(() => {
            // Offline or server unreachable: keep running the cached build.
          });
        }
      });
    })
    .catch((err) => {
      console.warn('Service worker registration failed:', err);
    });
}
