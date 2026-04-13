import React from 'react';
import ReactDOM from 'react-dom/client';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { ConfigProvider, theme } from 'antd';
import App from './App';
import './styles/global.css';

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      refetchOnWindowFocus: false,
      retry: 1,
    },
  },
});

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <ConfigProvider
      theme={{
        algorithm: theme.darkAlgorithm,
        token: {
          colorPrimary: '#06d6a0',
          colorBgContainer: '#18181b',
          colorBgElevated: '#1e1e22',
          colorBgLayout: '#09090b',
          colorBorder: '#3f3f46',
          colorBorderSecondary: '#27272a',
          colorText: '#fafafa',
          colorTextSecondary: '#a1a1aa',
          colorTextTertiary: '#71717a',
          borderRadius: 8,
          fontFamily: "'Outfit', sans-serif",
        },
        components: {
          Table: {
            headerBg: '#1e1e22',
            rowHoverBg: 'rgba(255, 255, 255, 0.03)',
            headerColor: '#71717a',
            colorBgContainer: '#18181b',
          },
          Card: {
            colorBgContainer: '#18181b',
          },
          Modal: {
            contentBg: '#18181b',
            headerBg: '#18181b',
          },
          Input: {
            colorBgContainer: '#1e1e22',
            colorBorder: '#3f3f46',
            activeBorderColor: '#06d6a0',
            hoverBorderColor: '#52525b',
          },
          Select: {
            colorBgContainer: '#1e1e22',
            colorBorder: '#3f3f46',
            optionActiveBg: 'rgba(255, 255, 255, 0.04)',
            optionSelectedBg: 'rgba(6, 214, 160, 0.12)',
          },
          DatePicker: {
            colorBgContainer: '#1e1e22',
            colorBorder: '#3f3f46',
          },
        },
      }}
    >
      <QueryClientProvider client={queryClient}>
        <App />
      </QueryClientProvider>
    </ConfigProvider>
  </React.StrictMode>,
);
