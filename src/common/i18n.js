import i18n from 'i18next'
import { initReactI18next } from 'react-i18next'

import LanguageDetector from 'i18next-browser-languagedetector'

import ENtranslation from './i18n/locales/en/translation.json'
import JAtranslation from './i18n/locales/ja/translation.json'
import KOtranslation from './i18n/locales/ko/translation.json'
import THtranslation from './i18n/locales/th/translation.json'
import zhHanstranslation from './i18n/locales/zh-Hans/translation.json'
import zhHanttranslation from './i18n/locales/zh-Hant/translation.json'
import TRtranslation from './i18n/locales/tr/translation.json'

const resources = {
    'zh-Hans': {
        translation: zhHanstranslation,
    },
    'zh-Hant': {
        translation: zhHanttranslation,
    },
    'en': {
        translation: ENtranslation,
    },
    'ja': {
        translation: JAtranslation,
    },
    'ko': {
        translation: KOtranslation,
    },
    'th': {
        translation: THtranslation,
    },
    'tr': {
        translation: TRtranslation,
    },
}

// All locale resources are bundled statically; no HTTP backend is needed, so
// the UI never waits on the network to have translations ready.
i18n.use(LanguageDetector)
    .use(initReactI18next)
    .init({
        resources,
        fallbackLng: 'en',

        interpolation: {
            escapeValue: false,
        },
        // prevent creating i18nextLng in localStorage for each domain
        detection: {
            caches: [],
        },
    })

export default i18n
