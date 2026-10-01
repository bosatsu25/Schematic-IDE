import React from 'react';
import ReactDOM from 'react-dom/client';

const App = () => {
  return (
    <main>
      <h1>Schematic IDE</h1>
      <p>Phase 0 workspace baseline initialized.</p>
    </main>
  );
};

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
