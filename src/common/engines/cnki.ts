import {
    AbstractFreeMT,
    aes128EcbEncryptPkcs7,
    bytesToBase64,
    fetchJson,
    runFreeTranslation,
    toUrlSafeBase64,
    utf8Bytes,
    cnkiTargetLang,
} from './free-mt'
import { IMessageRequest } from './interfaces'

// CNKI academic translation (dict.cnki.net, no API key, Chinese <-> English
// only). The `words` payload is URL-safe base64 of AES-128-ECB encrypted
// text under the site's fixed key; both requests must carry browser-like
// Origin/Referer or the WAF resets the connection.
const CNKI_AES_KEY = '4e87183cfd3a45fe'
const CNKI_BROWSER_HEADERS: Record<string, string> = {
    'Origin': 'https://dict.cnki.net',
    'Referer': 'https://dict.cnki.net/',
    'User-Agent':
        'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36',
}

export function buildCnkiWords(text: string): string {
    const encrypted = aes128EcbEncryptPkcs7(utf8Bytes(text), utf8Bytes(CNKI_AES_KEY))
    return toUrlSafeBase64(bytesToBase64(encrypted))
}

export function parseCnkiToken(data: unknown): string {
    const payload = data as { code?: unknown; data?: unknown }
    if (!payload || payload.code !== 200 || !payload.data) {
        throw new Error('Failed to obtain the CNKI translation token')
    }
    return String(payload.data)
}

export function parseCnkiText(data: unknown): string {
    const payload = (data as { data?: { isInputVerificationCode?: unknown; mResult?: unknown } })?.data
    if (payload && payload.isInputVerificationCode) {
        throw new Error(
            'CNKI is temporarily rate limiting requests; complete the human verification once on dict.cnki.net and retry'
        )
    }
    const out = payload?.mResult
    if (!out || !String(out).trim()) {
        throw new Error('CNKI translate returned no result')
    }
    return String(out)
}

// Token is reused for 4 minutes (zotero-pdf-translate caches 5; keep a
// safety margin), module-level so all translations share it.
let cnkiTokenCache = { value: '', at: 0 }

async function getCnkiToken(): Promise<string> {
    if (cnkiTokenCache.value && Date.now() - cnkiTokenCache.at < 4 * 60 * 1000) {
        return cnkiTokenCache.value
    }
    const data = await fetchJson('CNKI translate', 'https://dict.cnki.net/fyzs-front-api/getToken', {
        method: 'GET',
        headers: CNKI_BROWSER_HEADERS,
    })
    const token = parseCnkiToken(data)
    cnkiTokenCache = { value: token, at: Date.now() }
    return token
}

export class CNKI extends AbstractFreeMT {
    readonly serviceName = 'CNKI Translate'

    async sendMessage(req: IMessageRequest): Promise<void> {
        await runFreeTranslation(req, {
            label: this.serviceName,
            chunk: 750,
            max: 4000,
            makeChunkTranslator: async () => {
                cnkiTargetLang(req.toLang)
                const token = await getCnkiToken()
                return async (chunk) => {
                    const data = await fetchJson(
                        this.serviceName,
                        'https://dict.cnki.net/fyzs-front-api/translate/literaltranslation',
                        {
                            method: 'POST',
                            headers: {
                                'Content-Type': 'application/json;charset=UTF-8',
                                'Token': token,
                                ...CNKI_BROWSER_HEADERS,
                            },
                            body: JSON.stringify({ words: buildCnkiWords(chunk), translateType: null }),
                        }
                    )
                    return parseCnkiText(data)
                }
            },
        })
    }
}
