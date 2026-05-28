import React from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { ChevronLeft } from 'lucide-react';
import { Button } from '../components/ui/Button';
import GettingStartedZh from '../docs/user/getting-started.zh';
import GettingStartedEn from '../docs/user/getting-started.en';
import ApiKeysZh from '../docs/user/api-keys.zh.mdx';
import ApiKeysEn from '../docs/user/api-keys.en.mdx';
import BalanceZh from '../docs/user/balance.zh.mdx';
import BalanceEn from '../docs/user/balance.en.mdx';
import UsageZh from '../docs/user/usage.zh.mdx';
import UsageEn from '../docs/user/usage.en.mdx';
import ChannelsZh from '../docs/admin/channels.zh.mdx';
import ChannelsEn from '../docs/admin/channels.en.mdx';
import ProvidersZh from '../docs/admin/providers.zh.mdx';
import ProvidersEn from '../docs/admin/providers.en.mdx';
import ModelsZh from '../docs/admin/models.zh.mdx';
import ModelsEn from '../docs/admin/models.en.mdx';
import PricingPoliciesZh from '../docs/admin/pricing-policies.zh.mdx';
import PricingPoliciesEn from '../docs/admin/pricing-policies.en.mdx';
import RateLimitsZh from '../docs/admin/rate-limits.zh.mdx';
import RateLimitsEn from '../docs/admin/rate-limits.en.mdx';

const VALID_LANGS = ['zh', 'en'] as const;
type Lang = (typeof VALID_LANGS)[number];

const components: Record<string, Record<Lang, React.ComponentType<any>>> = {
  'getting-started': { zh: GettingStartedZh, en: GettingStartedEn },
  'api-keys': { zh: ApiKeysZh, en: ApiKeysEn },
  'balance': { zh: BalanceZh, en: BalanceEn },
  'usage': { zh: UsageZh, en: UsageEn },
  'channels': { zh: ChannelsZh, en: ChannelsEn },
  'providers': { zh: ProvidersZh, en: ProvidersEn },
  'models': { zh: ModelsZh, en: ModelsEn },
  'pricing-policies': { zh: PricingPoliciesZh, en: PricingPoliciesEn },
  'rate-limits': { zh: RateLimitsZh, en: RateLimitsEn },
};

export default function DocsPage() {
  const { lang, slug } = useParams<{ lang?: string; slug?: string }>();
  const navigate = useNavigate();
  const { t } = useTranslation();

  const validLang: Lang = VALID_LANGS.includes(lang as Lang) ? (lang as Lang) : 'zh';

  if (!slug) {
    navigate(`/docs/${validLang}/user/getting-started`, { replace: true });
    return null;
  }

  const Component = components[slug]?.[validLang];

  if (!Component) {
    return (
      <div className="text-center py-20">
        <h1 className="text-2xl font-bold mb-4">404</h1>
        <p className="text-base-content/60 mb-4">{t('docs.notFound')}</p>
        <Button onClick={() => navigate(`/docs/${validLang}/user/getting-started`)}>
          <ChevronLeft className="h-4 w-4" /> {t('docs.goHome')}
        </Button>
      </div>
    );
  }

  return (
    <article className="prose prose-sm max-w-none
      prose-headings:font-semibold prose-headings:tracking-tight
      prose-h1:text-2xl prose-h1:mb-4
      prose-h2:text-xl prose-h2:mt-8 prose-h2:mb-3
      prose-h3:text-base prose-h3:mt-6 prose-h3:mb-2
      prose-p:text-base prose-p:leading-7 prose-p:text-base-content/80
      prose-strong:text-base-content
      prose-code:text-primary prose-code:before:content-[''] prose-code:after:content-['']
      prose-code:bg-primary/10 prose-code:px-1.5 prose-code:py-0.5 prose-code:rounded prose-code:text-sm
      prose-pre:bg-base-300 prose-pre:border prose-pre:border-base-300/40 prose-pre:rounded-lg
      prose-a:text-primary prose-a:no-underline hover:prose-a:underline
      prose-li:text-base-content/80
      prose-ol:pl-6 prose-ul:pl-6
    ">
      <Component />
    </article>
  );
}
