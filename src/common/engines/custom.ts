import { urlJoin } from 'url-join-ts'
import { getUniversalFetch } from '../universal-fetch'
import { getSettings } from '../utils'
import { AbstractOpenAI } from './abstract-openai'
import { IModel } from './interfaces'

// A fully user-configured OpenAI-compatible endpoint. The base URL, path and
// API key all come from settings, so this works with any gateway that speaks
// the OpenAI Chat Completions (or Responses) protocol: self-hosted vLLM,
// one-api/new-api relays, national-cloud mirrors, OpenCode, Kimi Coding, etc.
// The API key is optional because local gateways often run without auth.
export class Custom extends AbstractOpenAI {
    supportCustomModel(): boolean {
        return true
    }

    async listModels(apiKey_: string | undefined): Promise<IModel[]> {
        let apiKey = apiKey_
        if (apiKey === undefined) {
            apiKey = await this.getAPIKey()
        }
        const apiURL = await this.getAPIURL()
        const apiURLPath = await this.getAPIURLPath()
        if (!apiURL) {
            return []
        }
        // Derive the models endpoint from the chat endpoint: strip the
        // completion tail from the joined URL and append /models. This accepts
        // both "https://host/v1" + "/chat/completions" and
        // "https://host/v1/chat/completions" + "/" style configurations.
        const chatURL = urlJoin(apiURL, apiURLPath || '/')
        const base = chatURL.replace(/\/(chat\/completions|responses|completions)\/?$/i, '')
        const url = urlJoin(base, '/models')
        const fetcher = getUniversalFetch()
        const headers: Record<string, string> = {
            'Content-Type': 'application/json',
        }
        // Some self-hosted gateways run without authentication, so only send
        // the Authorization header when a key is configured.
        if (apiKey) {
            headers['Authorization'] = `Bearer ${apiKey}`
        }
        const response = await fetcher(url, {
            method: 'GET',
            headers,
        })
        if (response.status !== 200) {
            if (response.status === 401 || response.status === 403) {
                throw new Error('Invalid API key')
            }
            if (response.status === 404) {
                throw new Error('Invalid API URL')
            }
            throw new Error(`Failed to list models: ${response.statusText}`)
        }
        const json = await response.json()
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        return (json.data ?? []).map((model: any) => {
            return {
                id: model.id,
                name: model.id,
            }
        })
    }

    async getAPIModel(): Promise<string> {
        const settings = await getSettings()
        return settings.customAPIModel
    }
    async getAPIKey(): Promise<string> {
        const settings = await getSettings()
        return settings.customAPIKey
    }
    async getAPIURL(): Promise<string> {
        const settings = await getSettings()
        return settings.customAPIURL
    }
    async getAPIURLPath(): Promise<string> {
        const settings = await getSettings()
        // If the base URL already carries the full endpoint path (some
        // gateways hand out complete URLs like https://host/v1/chat/completions),
        // don't append the path a second time.
        if (/\/(chat\/completions|responses|completions)\/?$/i.test(settings.customAPIURL ?? '')) {
            return '/'
        }
        return settings.customAPIURLPath
    }
}
