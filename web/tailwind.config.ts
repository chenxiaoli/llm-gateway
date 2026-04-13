import type { Config } from 'tailwindcss';

export default {
  content: ['./index.html', './src/**/*.{ts,tsx}'],
  theme: {
    extend: {
      colors: {
        accent: {
          DEFAULT: '#06d6a0',
          hover: '#34d399',
          dim: 'rgba(6, 214, 160, 0.12)',
          glow: 'rgba(6, 214, 160, 0.25)',
        },
        surface: {
          DEFAULT: '#111111',
          elevated: '#141414',
        },
        danger: '#ef4444',
        warning: '#f59e0b',
        info: '#3b82f6',
        purple: '#a855f7',
      },
      fontFamily: {
        sans: ['Outfit', 'system-ui', 'sans-serif'],
        mono: ['JetBrains Mono', 'monospace'],
      },
      borderRadius: {
        DEFAULT: '8px',
      },
    },
  },
  plugins: [],
} satisfies Config;
