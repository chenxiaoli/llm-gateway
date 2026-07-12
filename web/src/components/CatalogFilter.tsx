import { useTranslation } from 'react-i18next';

export type CatalogScope = 'all' | 'platform' | 'ours';

interface Props {
  value: CatalogScope;
  onChange: (s: CatalogScope) => void;
  /** Hide the "Ours" option when the user can't create org-private entries */
  showOrgOption: boolean;
}

export function CatalogFilter({ value, onChange, showOrgOption }: Props) {
  const { t } = useTranslation();
  const options: CatalogScope[] = showOrgOption
    ? ['all', 'platform', 'ours']
    : ['all', 'platform'];
  return (
    <div className="inline-flex rounded-lg border border-base-300/40 bg-base-200/30 p-0.5">
      {options.map((s) => (
        <button
          key={s}
          type="button"
          onClick={() => onChange(s)}
          className={`px-3 py-1.5 text-xs font-semibold rounded-md transition-colors cursor-pointer ${
            value === s
              ? 'bg-base-100 text-base-content shadow-sm'
              : 'text-base-content/55 hover:text-base-content/80'
          }`}
        >
          {t(`catalog.filter.${s}`)}
        </button>
      ))}
    </div>
  );
}
