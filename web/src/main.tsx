import React from 'react';
import ReactDOM from 'react-dom/client';
import { QueryClientProvider } from '@tanstack/react-query';
import { Toaster } from 'sonner';
import './i18n';
import App from './App';
import './styles/global.css';
import { queryClient } from './lib/queryClient';

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <QueryClientProvider client={queryClient}>
      <App />
      <Toaster
        position="top-right"
        theme="dark"
        toastOptions={{
          style: {
            background: '#141414',
            border: '1px solid #1e1e1e',
            color: '#ededed',
          },
        }}
      />
    </QueryClientProvider>
  </React.StrictMode>,
);
