import { useTranslation } from 'react-i18next';
import { motion } from 'framer-motion';

/**
 * Rendered when a catalog filter (`Platform` / `Ours`) yields zero rows but
 * the unfiltered dataset is non-empty. Distinct from the page-level
 * "create first" empty state — that one only renders when the user has no
 * entries at all.
 */
export function CatalogNoMatches() {
  const { t } = useTranslation();
  return (
    <motion.div
      initial={{ opacity: 0, y: 12 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.4 }}
      className="flex flex-col items-center justify-center py-20 px-4 text-center"
    >
      <h3 className="text-lg font-semibold text-base-content/60 mb-1.5">
        {t('catalog.noMatches.title')}
      </h3>
      <p className="text-sm text-base-content/40 max-w-sm leading-relaxed">
        {t('catalog.noMatches.description')}
      </p>
    </motion.div>
  );
}
