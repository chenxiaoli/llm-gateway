import { create } from 'zustand';
import { getAuthConfig } from '../api/auth';

interface CurrencyState {
  currency: string;
  symbol: string;
  init: () => Promise<void>;
  setCurrency: (currency: string) => void;
}

function currencyToSymbol(c: string): string {
  switch (c) {
    case 'CNY': return '¥';
    case 'USD': return '$';
    default: return '$';
  }
}

export const useCurrencyStore = create<CurrencyState>((set) => ({
  currency: 'USD',
  symbol: '$',
  init: async () => {
    try {
      const config = await getAuthConfig();
      const currency = config.currency || 'USD';
      set({ currency, symbol: currencyToSymbol(currency) });
    } catch {
      // Default to USD on error
    }
  },
  setCurrency: (currency: string) => {
    set({ currency, symbol: currencyToSymbol(currency) });
  },
}));

/** Format a numeric amount with the given currency symbol */
export function formatCurrency(amount: number, symbol: string, decimals = 4): string {
  return `${symbol}${amount.toFixed(decimals)}`;
}
