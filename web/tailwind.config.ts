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
      fontSize: {
        'xs': ['0.9375rem', { lineHeight: '1.2' }],
        'sm': ['1rem', { lineHeight: '1.5' }],
        'base': ['1.125rem', { lineHeight: '1.6' }],
        'md': ['1.25rem', { lineHeight: '1.6' }],
        'lg': ['1.5rem', { lineHeight: '1.5' }],
        'xl': ['1.75rem', { lineHeight: '1.4' }],
        '2xl': ['2rem', { lineHeight: '1.4' }],
        '3xl': ['2.25rem', { lineHeight: '1.3' }],
        '4xl': ['2.5rem', { lineHeight: '1.2' }],
      },
    },
  },
  plugins: [],
} satisfies Config;
