import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import en from './en.json';
import zh from './zh.json';

const savedLanguage = localStorage.getItem('i18n-language');

i18n.use(initReactI18next).init({
  resources: {
    en: { translation: en },
    zh: { translation: zh },
  },
  lng: savedLanguage || (navigator.language.startsWith('zh') ? 'zh' : 'en'),
  fallbackLng: 'en',
  supportedLngs: ['en', 'zh'],
  interpolation: {
    escapeValue: false,
  },
});

export default i18n;
