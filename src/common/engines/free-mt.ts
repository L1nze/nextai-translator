import { getUniversalFetch } from '../universal-fetch'
import { IEngine, IMessageRequest, IModel } from './interfaces'

// Shared plumbing for the free (no-API-key) machine-translation engines,
// ported from the LitBoard project's adaptation of
// windingwind/zotero-pdf-translate (MIT). Covers what the free endpoints
// need: sentence-aware chunking, the Google web `tk` signature, AES-128-ECB
// encryption for CNKI, target-language mapping, and the shared request
// orchestration. Per-service request building / response parsing lives in
// the engine files.

// ---------- sentence-aware chunking ----------

const SENTENCE_CLOSERS = '"\'\u201d\u2019」』）)]】》'

function isSentenceEnd(text: string, index: number): boolean {
    const ch = text.charAt(index)
    if (
        ch === '\n' ||
        ch === '。' ||
        ch === '！' ||
        ch === '？' ||
        ch === '!' ||
        ch === '?' ||
        ch === ';' ||
        ch === '；'
    ) {
        return true
    }
    // English period only closes a sentence when followed by whitespace,
    // a quote or the end of text (abbreviations and decimals stay intact).
    if (ch === '.') {
        const next = index + 1 < text.length ? text.charAt(index + 1) : ''
        return next === '' || /[\s"'’”)\]}]/.test(next)
    }
    return false
}

export function splitSentences(text: string): string[] {
    const pieces: string[] = []
    let start = 0
    for (let i = 0; i < text.length; i++) {
        if (!isSentenceEnd(text, i)) {
            continue
        }
        let end = i + 1
        while (end < text.length && SENTENCE_CLOSERS.indexOf(text.charAt(end)) >= 0) {
            end++
        }
        pieces.push(text.slice(start, end))
        start = end
        i = end - 1
    }
    if (start < text.length) {
        pieces.push(text.slice(start))
    }
    return pieces
}

export function splitChunks(text: string, max: number): string[] {
    const source = String(text ?? '')
    if (!(max > 0) || source.length <= max) {
        return [source]
    }
    const chunks: string[] = []
    let current = ''
    for (const piece of splitSentences(source)) {
        if (!piece) {
            continue
        }
        if (piece.length > max) {
            if (current) {
                chunks.push(current)
                current = ''
            }
            for (let j = 0; j < piece.length; j += max) {
                chunks.push(piece.slice(j, j + max))
            }
            continue
        }
        if (current.length + piece.length > max) {
            chunks.push(current)
            current = piece
        } else {
            current += piece
        }
    }
    if (current) {
        chunks.push(current)
    }
    return chunks
}

// ---------- Google web endpoint `tk` signature ----------

// tk algorithm ported from windingwind/zotero-pdf-translate (MIT), itself
// from the public Google Translate web script; the shift strings are fixed
// constants of that variant and do not depend on a server-issued TKK.
function googleRl(a: number, b: string): number {
    for (let c = 0; c < b.length - 2; c += 3) {
        const d = b.charAt(c + 2)
        const shift = d >= 'a' ? d.charCodeAt(0) - 87 : Number(d)
        const dd = b.charAt(c + 1) === '+' ? a >>> shift : (a << shift) >>> 0
        a = b.charAt(c) === '+' ? (a + dd) & 4294967295 : (a ^ dd) >>> 0
    }
    return a
}

export function googleTk(text: string): string {
    const SHIFT_A = '+-a^+6'
    const SHIFT_B = '+-3^+b+-f'
    const bytes: number[] = []
    for (let i = 0; i < text.length; i++) {
        let m = text.charCodeAt(i)
        if (m < 128) {
            bytes.push(m)
        } else if (m < 2048) {
            bytes.push((m >> 6) | 192, (m & 63) | 128)
        } else if ((m & 64512) === 55296 && i + 1 < text.length && (text.charCodeAt(i + 1) & 64512) === 56320) {
            m = 65536 + ((m & 1023) << 10) + (text.charCodeAt(++i) & 1023)
            bytes.push((m >> 18) | 240, ((m >> 12) & 63) | 128, ((m >> 6) & 63) | 128, (m & 63) | 128)
        } else {
            bytes.push((m >> 12) | 224, ((m >> 6) & 63) | 128, (m & 63) | 128)
        }
    }
    let a = 406644
    for (const b of bytes) {
        a += b
        a = googleRl(a, SHIFT_A)
    }
    a = googleRl(a, SHIFT_B)
    a = (a ^ 3293161072) >>> 0
    a %= 1000000
    return String(a) + '.' + (a ^ 406644)
}

// ---------- AES-128-ECB (encrypt only, PKCS#7) for CNKI ----------

// WebCrypto deliberately does not expose ECB, so this is a compact
// encrypt-only AES-128 for the fixed CNKI request key. Cross-verified
// against Node's crypto implementation in free-mt.spec.ts.
const AES_SBOX = new Uint8Array([
    0x63, 0x7c, 0x77, 0x7b, 0xf2, 0x6b, 0x6f, 0xc5, 0x30, 0x01, 0x67, 0x2b, 0xfe, 0xd7, 0xab, 0x76, 0xca, 0x82, 0xc9,
    0x7d, 0xfa, 0x59, 0x47, 0xf0, 0xad, 0xd4, 0xa2, 0xaf, 0x9c, 0xa4, 0x72, 0xc0, 0xb7, 0xfd, 0x93, 0x26, 0x36, 0x3f,
    0xf7, 0xcc, 0x34, 0xa5, 0xe5, 0xf1, 0x71, 0xd8, 0x31, 0x15, 0x04, 0xc7, 0x23, 0xc3, 0x18, 0x96, 0x05, 0x9a, 0x07,
    0x12, 0x80, 0xe2, 0xeb, 0x27, 0xb2, 0x75, 0x09, 0x83, 0x2c, 0x1a, 0x1b, 0x6e, 0x5a, 0xa0, 0x52, 0x3b, 0xd6, 0xb3,
    0x29, 0xe3, 0x2f, 0x84, 0x53, 0xd1, 0x00, 0xed, 0x20, 0xfc, 0xb1, 0x5b, 0x6a, 0xcb, 0xbe, 0x39, 0x4a, 0x4c, 0x58,
    0xcf, 0xd0, 0xef, 0xaa, 0xfb, 0x43, 0x4d, 0x33, 0x85, 0x45, 0xf9, 0x02, 0x7f, 0x50, 0x3c, 0x9f, 0xa8, 0x51, 0xa3,
    0x40, 0x8f, 0x92, 0x9d, 0x38, 0xf5, 0xbc, 0xb6, 0xda, 0x21, 0x10, 0xff, 0xf3, 0xd2, 0xcd, 0x0c, 0x13, 0xec, 0x5f,
    0x97, 0x44, 0x17, 0xc4, 0xa7, 0x7e, 0x3d, 0x64, 0x5d, 0x19, 0x73, 0x60, 0x81, 0x4f, 0xdc, 0x22, 0x2a, 0x90, 0x88,
    0x46, 0xee, 0xb8, 0x14, 0xde, 0x5e, 0x0b, 0xdb, 0xe0, 0x32, 0x3a, 0x0a, 0x49, 0x06, 0x24, 0x5c, 0xc2, 0xd3, 0xac,
    0x62, 0x91, 0x95, 0xe4, 0x79, 0xe7, 0xc8, 0x37, 0x6d, 0x8d, 0xd5, 0x4e, 0xa9, 0x6c, 0x56, 0xf4, 0xea, 0x65, 0x7a,
    0xae, 0x08, 0xba, 0x78, 0x25, 0x2e, 0x1c, 0xa6, 0xb4, 0xc6, 0xe8, 0xdd, 0x74, 0x1f, 0x4b, 0xbd, 0x8b, 0x8a, 0x70,
    0x3e, 0xb5, 0x66, 0x48, 0x03, 0xf6, 0x0e, 0x61, 0x35, 0x57, 0xb9, 0x86, 0xc1, 0x1d, 0x9e, 0xe1, 0xf8, 0x98, 0x11,
    0x69, 0xd9, 0x8e, 0x94, 0x9b, 0x1e, 0x87, 0xe9, 0xce, 0x55, 0x28, 0xdf, 0x8c, 0xa1, 0x89, 0x0d, 0xbf, 0xe6, 0x42,
    0x68, 0x41, 0x99, 0x2d, 0x0f, 0xb0, 0x54, 0xbb, 0x16,
])
const AES_RCON = [0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, 0x1b, 0x36]

function xtime(b: number): number {
    const shifted = b << 1
    return (shifted ^ (b & 0x80 ? 0x1b : 0)) & 0xff
}

// Standard AES-128 key schedule: 11 round keys, word-wise expansion
// (a word is 4 bytes; words 1..3 of each round derive from the previous
// word of the SAME round, word 0 from RotWord/SubWord/Rcon).
function aesExpandKey128(key: Uint8Array): Uint8Array[] {
    const roundKeys: Uint8Array[] = [new Uint8Array(key)]
    for (let round = 1; round <= 10; round++) {
        const prev = roundKeys[round - 1]
        const next = new Uint8Array(16)
        const t0 = prev[12]
        const t1 = prev[13]
        const t2 = prev[14]
        const t3 = prev[15]
        next[0] = prev[0] ^ AES_SBOX[t1] ^ AES_RCON[round - 1]
        next[1] = prev[1] ^ AES_SBOX[t2]
        next[2] = prev[2] ^ AES_SBOX[t3]
        next[3] = prev[3] ^ AES_SBOX[t0]
        for (let word = 1; word < 4; word++) {
            for (let b = 0; b < 4; b++) {
                next[word * 4 + b] = prev[word * 4 + b] ^ next[(word - 1) * 4 + b]
            }
        }
        roundKeys.push(next)
    }
    return roundKeys
}

function aesEncryptBlock(block: Uint8Array, roundKeys: Uint8Array[]): Uint8Array {
    const state = new Uint8Array(block)
    const addRoundKey = (round: number) => {
        for (let i = 0; i < 16; i++) {
            state[i] ^= roundKeys[round][i]
        }
    }
    addRoundKey(0)
    for (let round = 1; round <= 10; round++) {
        // SubBytes
        for (let i = 0; i < 16; i++) {
            state[i] = AES_SBOX[state[i]]
        }
        // ShiftRows (block is column-major: index = row + 4 * col)
        let t = state[1]
        state[1] = state[5]
        state[5] = state[9]
        state[9] = state[13]
        state[13] = t
        t = state[2]
        state[2] = state[10]
        state[10] = t
        t = state[6]
        state[6] = state[14]
        state[14] = t
        t = state[15]
        state[15] = state[11]
        state[11] = state[7]
        state[7] = state[3]
        state[3] = t
        if (round < 10) {
            // MixColumns
            for (let col = 0; col < 4; col++) {
                const o = col * 4
                const a0 = state[o]
                const a1 = state[o + 1]
                const a2 = state[o + 2]
                const a3 = state[o + 3]
                state[o] = xtime(a0) ^ (xtime(a1) ^ a1) ^ a2 ^ a3
                state[o + 1] = a0 ^ xtime(a1) ^ (xtime(a2) ^ a2) ^ a3
                state[o + 2] = a0 ^ a1 ^ xtime(a2) ^ (xtime(a3) ^ a3)
                state[o + 3] = xtime(a0) ^ a0 ^ a1 ^ a2 ^ xtime(a3)
            }
        }
        addRoundKey(round)
    }
    return state
}

export function aes128EcbEncryptPkcs7(plain: Uint8Array, key: Uint8Array): Uint8Array {
    const roundKeys = aesExpandKey128(key)
    const padLen = 16 - (plain.length % 16)
    const padded = new Uint8Array(plain.length + padLen)
    padded.set(plain)
    padded.fill(padLen, plain.length)
    const out = new Uint8Array(padded.length)
    for (let off = 0; off < padded.length; off += 16) {
        out.set(aesEncryptBlock(padded.slice(off, off + 16), roundKeys), off)
    }
    return out
}

export function utf8Bytes(text: string): Uint8Array {
    return new TextEncoder().encode(text)
}

export function bytesToBase64(bytes: Uint8Array): string {
    let binary = ''
    for (let i = 0; i < bytes.length; i++) {
        binary += String.fromCharCode(bytes[i])
    }
    return btoa(binary)
}

export function toUrlSafeBase64(base64Text: string): string {
    return String(base64Text ?? '')
        .replace(/\//g, '_')
        .replace(/\+/g, '-')
}

// ---------- target-language mapping ----------

// App-specific codes that never match a service code directly.
const CHINESE_FAMILY = new Set(['zh-Hans', 'zh-Hant', 'yue', 'lzh', 'jdbhw', 'xdbhw'])

function baseLang(code: string): string {
    if (CHINESE_FAMILY.has(code)) {
        return 'zh'
    }
    if (code === 'ko-banmal') {
        return 'ko'
    }
    return code.split('-')[0]
}

function mapLang(code: string | undefined, variants: Record<string, string>, supported?: Set<string>): string {
    if (!code) {
        throw new Error('Target language is unknown')
    }
    if (variants[code]) {
        return variants[code]
    }
    const base = baseLang(code)
    if (!supported || supported.has(base)) {
        return base
    }
    throw new Error(`This service does not support translating to "${code}"`)
}

export const googleTargetLang = (code: string | undefined) =>
    mapLang(code, {
        'zh-Hans': 'zh-CN',
        'zh-Hant': 'zh-TW',
        'yue': 'zh-TW',
        'lzh': 'zh-CN',
        'jdbhw': 'zh-CN',
        'xdbhw': 'zh-CN',
    })

const LANGS_ZH: Record<string, string> = {
    'zh-Hans': 'zh',
    'zh-Hant': 'zh',
    'yue': 'zh',
    'lzh': 'zh',
    'jdbhw': 'zh',
    'xdbhw': 'zh',
}

const COMMON_LANGS = new Set([
    'zh',
    'en',
    'ja',
    'ko',
    'fr',
    'de',
    'es',
    'it',
    'ru',
    'pt',
    'th',
    'vi',
    'id',
    'ms',
    'ar',
    'hi',
    'tr',
    'nl',
    'pl',
    'uk',
    'sv',
    'da',
    'fi',
    'no',
    'cs',
    'ro',
    'hu',
    'el',
    'he',
    'fa',
])
export const transmartTargetLang = (code: string | undefined) => mapLang(code, LANGS_ZH, COMMON_LANGS)
export const volcTargetLang = (code: string | undefined) => mapLang(code, LANGS_ZH, COMMON_LANGS)

// CNKI auto-detects direction and only serves Chinese <-> English.
export const cnkiTargetLang = (code: string | undefined) => {
    if (!code) {
        throw new Error('Target language is unknown')
    }
    const base = baseLang(code)
    if (base === 'zh' || base === 'en') {
        return base
    }
    throw new Error('CNKI translate only supports Chinese <-> English')
}

export const myMemoryTargetLang = (code: string | undefined) =>
    mapLang(code, {
        'zh-Hans': 'zh-CN',
        'zh-Hant': 'zh-TW',
        'yue': 'zh-CN',
        'lzh': 'zh-CN',
        'jdbhw': 'zh-CN',
        'xdbhw': 'zh-CN',
    })

// ---------- request orchestration ----------

// Common shape of the free-MT engines: no key, no model list, always
// "logged in"; only sendMessage differs per service.
export abstract class AbstractFreeMT implements IEngine {
    abstract readonly serviceName: string
    async checkLogin(): Promise<boolean> {
        return true
    }
    isLocal(): boolean {
        return false
    }
    supportCustomModel(): boolean {
        return false
    }
    async getModel(): Promise<string> {
        return this.serviceName
    }
    async listModels(): Promise<IModel[]> {
        return []
    }
    abstract sendMessage(req: IMessageRequest): Promise<void>
}

export interface IFreeTranslateOptions {
    label: string
    chunk: number
    max: number
    // Resolves the target language (throwing early for unsupported ones)
    // and returns the per-chunk translator used by the pipeline.
    makeChunkTranslator: (signal: AbortSignal) => Promise<(chunk: string) => Promise<string>>
}

// Drives one free-MT translation through the shared pipeline: mode check,
// length check, chunking, polite sequential requests, incremental emission
// through onMessage. All failures surface via onError + onFinished('error').
export async function runFreeTranslation(req: IMessageRequest, opts: IFreeTranslateOptions): Promise<void> {
    const { label, chunk, max, makeChunkTranslator } = opts
    try {
        if (req.mode && req.mode !== 'translate') {
            throw new Error(
                `${label} only supports translation. Switch to an LLM provider (e.g. OpenAI) for polishing, summarizing or code explanation.`
            )
        }
        const text = (req.sourceText ?? '').trim()
        if (!text) {
            throw new Error('Nothing to translate')
        }
        if (text.length > max) {
            throw new Error(`${label} cannot translate more than ${max} characters at once (got ${text.length})`)
        }
        const translateChunk = await makeChunkTranslator(req.signal)
        const chunks = splitChunks(text, chunk)
        for (let i = 0; i < chunks.length; i++) {
            if (req.signal.aborted) {
                req.onFinished('stop')
                return
            }
            // Polite pacing between sequential chunks, like zotero-pdf-translate.
            if (i > 0) {
                await new Promise((resolve) => setTimeout(resolve, 200))
            }
            const translated = await translateChunk(chunks[i])
            if (translated) {
                await req.onMessage({ content: translated, role: 'assistant' })
            }
        }
        req.onFinished('stop')
    } catch (e) {
        req.onError(e instanceof Error ? e.message : String(e))
        req.onFinished('error')
    }
}

export async function fetchJson(label: string, url: string, init: RequestInit): Promise<unknown> {
    const fetcher = getUniversalFetch()
    const response = await fetcher(url, init)
    if (!response.ok) {
        if (response.status === 429) {
            throw new Error(`${label} is rate limiting requests, please retry later`)
        }
        throw new Error(`${label} request failed with status ${response.status}`)
    }
    try {
        return await response.json()
    } catch {
        throw new Error(`${label} returned an unparsable response`)
    }
}
