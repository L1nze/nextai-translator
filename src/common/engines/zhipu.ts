import { urlJoin } from 'url-join-ts'
import { getUniversalFetch } from '../universal-fetch'
import { getSettings } from '../utils'
import { AbstractOpenAI } from './abstract-openai'
import { IModel } from './interfaces'

// Zhipu AI open platform serves GLM models over an OpenAI-compatible API.
// The default endpoint is https://open.bigmodel.cn/api/paas/v4/chat/completions;
// the international Z.ai gateway (https://api.z.ai/api/paas/v4) speaks the same
// protocol, so the base URL is user-editable to switch between the two.
export class Zhipu extends AbstractOpenAI {
    async listModels(apiKey_: string | undefined): Promise<IModel[]> {
        let apiKey = apiKey_
        if (!apiKey) {
            apiKey = await this.getAPIKey()
        }
        const apiURL = await this.getAPIURL()
        if (!apiKey || !apiURL) {
            return []
        }
        const url = urlJoin(apiURL, '/paas/v4/models')
        const fetcher = getUniversalFetch()
        const response = await fetcher(url, {
            method: 'GET',
            headers: {
                'Content-Type': 'application/json',
                'Authorization': `Bearer ${apiKey}`,
            },
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
        return (json.data ?? json.models ?? []).map((model: any) => {
            const id = model.id ?? model.model ?? model
            return {
                id,
                name: id,
            }
        })
    }

    async getAPIModel(): Promise<string> {
        const settings = await getSettings()
        return settings.zhipuAPIModel
    }
    async getAPIKey(): Promise<string> {
        const settings = await getSettings()
        return settings.zhipuAPIKey
    }
    async getAPIURL(): Promise<string> {
        const settings = await getSettings()
        return settings.zhipuAPIURL || 'https://open.bigmodel.cn/api'
    }
    async getAPIURLPath(): Promise<string> {
        return '/paas/v4/chat/completions'
    }
}
