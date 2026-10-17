import { RiOpenaiFill, RiTranslate2 } from 'react-icons/ri'
import { Azure } from './azure'
import { ChatGPT } from './chatgpt'
import { Gemini } from './gemini'
import { IEngine } from './interfaces'
import { MiniMax } from './minimax'
import { Moonshot } from './moonshot'
import { OpenAI } from './openai'
import { IconType } from 'react-icons'
import { VscAzureDevops } from 'react-icons/vsc'
import { FaBook, FaDatabase, FaGoogle } from 'react-icons/fa'
import { GiArtificialIntelligence, GiVolcano } from 'react-icons/gi'
import { Ollama } from './ollama'
import { OllamaIcon } from '@/common/components/icons/OllamaIcon'
import { MoonshotIcon } from '@/common/components/icons/MoonshotIcon'
import { ClaudeIcon } from '@/common/components/icons/ClaudeIcon'
import { Groq } from './groq'
import { GroqIcon } from '@/common/components/icons/GroqIcon'
import { KimiIcon } from '@/common/components/icons/KimiIcon'
import { Claude } from './claude'
import { Kimi } from './kimi'
import { ChatGLMIcon } from '@/common/components/icons/ChatGLMIcon'
import { ChatGLM } from './chatglm'
import { CohereIcon } from '@/common/components/icons/CohereIcon'
import { Cohere } from './cohere'
import { DeepSeekIcon } from '@/common/components/icons/DeepSeekIcon'
import { DeepSeek } from './deepseek'
import { CerebrasIcon } from '@/common/components/icons/CerebrasIcon'
import { Cerebras } from './cerebras'
import { TeamoRouterIcon } from '@/common/components/icons/TeamoRouterIcon'
import { TeamoRouter } from './teamorouter'
import { OpenRouterIcon } from '@/common/components/icons/OpenRouterIcon'
import { OpenRouter } from './openrouter'
import { LiteLLMIcon } from '@/common/components/icons/LiteLLMIcon'
import { LiteLLM } from './litellm'
import { SiliconFlow } from './siliconflow'
import { DashScope } from './dashscope'
import { Zhipu } from './zhipu'
import { MiMo } from './mimo'
import { Custom } from './custom'
import { OpenCode } from './opencode'
import { GoogleTranslate } from './google-translate'
import { Transmart } from './transmart'
import { Volcengine } from './volcengine'
import { CNKI } from './cnki'
import { MyMemory } from './mymemory'
import { SiliconFlowIcon } from '@/common/components/icons/SiliconFlowIcon'
import { DashScopeIcon } from '@/common/components/icons/DashScopeIcon'
import { ZhipuIcon } from '@/common/components/icons/ZhipuIcon'
import { MiMoIcon } from '@/common/components/icons/MiMoIcon'
import { CustomIcon } from '@/common/components/icons/CustomIcon'
import { OpenCodeIcon } from '@/common/components/icons/OpenCodeIcon'
import { TransmartIcon } from '@/common/components/icons/TransmartIcon'

export type Provider =
    | 'OpenAI'
    | 'ChatGPT'
    | 'Azure'
    | 'MiniMax'
    | 'Moonshot'
    | 'Gemini'
    | 'Ollama'
    | 'Groq'
    | 'Claude'
    | 'Kimi'
    | 'ChatGLM'
    | 'Cohere'
    | 'DeepSeek'
    | 'Cerebras'
    | 'TeamoRouter'
    | 'OpenRouter'
    | 'LiteLLM'
    | 'SiliconFlow'
    | 'DashScope'
    | 'Zhipu'
    | 'MiMo'
    | 'Custom'
    | 'OpenCode'
    | 'GoogleTranslate'
    | 'Transmart'
    | 'Volcengine'
    | 'CNKI'
    | 'MyMemory'

export const engineIcons: Record<Provider, IconType> = {
    OpenAI: RiOpenaiFill,
    ChatGPT: RiOpenaiFill,
    Azure: VscAzureDevops,
    MiniMax: GiArtificialIntelligence,
    Moonshot: MoonshotIcon,
    Gemini: FaGoogle,
    Ollama: OllamaIcon,
    Groq: GroqIcon,
    Claude: ClaudeIcon,
    Kimi: KimiIcon,
    ChatGLM: ChatGLMIcon,
    Cohere: CohereIcon,
    DeepSeek: DeepSeekIcon,
    Cerebras: CerebrasIcon,
    TeamoRouter: TeamoRouterIcon,
    OpenRouter: OpenRouterIcon,
    LiteLLM: LiteLLMIcon,
    SiliconFlow: SiliconFlowIcon,
    DashScope: DashScopeIcon,
    Zhipu: ZhipuIcon,
    MiMo: MiMoIcon,
    Custom: CustomIcon,
    OpenCode: OpenCodeIcon,
    GoogleTranslate: RiTranslate2,
    Transmart: TransmartIcon,
    Volcengine: GiVolcano,
    CNKI: FaBook,
    MyMemory: FaDatabase,
}

export const providerToEngine: Record<Provider, { new (): IEngine }> = {
    OpenAI: OpenAI,
    ChatGPT: ChatGPT,
    Azure: Azure,
    MiniMax: MiniMax,
    Moonshot: Moonshot,
    Gemini: Gemini,
    Ollama: Ollama,
    Groq: Groq,
    Claude: Claude,
    Kimi: Kimi,
    ChatGLM: ChatGLM,
    Cohere: Cohere,
    DeepSeek: DeepSeek,
    Cerebras: Cerebras,
    TeamoRouter: TeamoRouter,
    OpenRouter: OpenRouter,
    LiteLLM: LiteLLM,
    SiliconFlow: SiliconFlow,
    DashScope: DashScope,
    Zhipu: Zhipu,
    MiMo: MiMo,
    Custom: Custom,
    OpenCode: OpenCode,
    GoogleTranslate: GoogleTranslate,
    Transmart: Transmart,
    Volcengine: Volcengine,
    CNKI: CNKI,
    MyMemory: MyMemory,
}

export function getEngine(provider: Provider): IEngine {
    const cls = providerToEngine[provider]
    return new cls()
}

// Free machine-translation providers. They translate the raw `sourceText`
// directly and never participate in the LLM prompt pipeline, so summarizing
// / polishing / analysis actions must use a regular LLM provider instead.
export const FREE_MT_PROVIDERS = new Set<Provider>(['GoogleTranslate', 'Transmart', 'Volcengine', 'CNKI', 'MyMemory'])
