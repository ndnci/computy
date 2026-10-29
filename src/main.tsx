import React from 'react';
import { createRoot } from 'react-dom/client';
import '@fontsource-variable/dm-sans';
import '@fontsource-variable/sora';
import './styles.css';
import './refinements.css';
import './monitor.css';
import App from './App';
import MonitorPopover from './MonitorPopover';
import { invoke } from '@tauri-apps/api/core';
document.addEventListener('click', (event) => {
  const link = (event.target as Element)?.closest<HTMLAnchorElement>('a[href^="https://"]');
  if (link && window.__TAURI_INTERNALS__) {
    event.preventDefault();
    void invoke('open_external', { url: link.href }).catch(console.error);
  }
});
createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    {new URLSearchParams(location.search).has('popover') ? <MonitorPopover /> : <App />}
  </React.StrictMode>,
);
