export interface Provider {
  id: string
  name: string
  base_url: string
  auth_type: string
  auth_header: string
  endpoints: ProviderEndpoint[]
  api_keys: ApiKeyInfo[]
}

export interface ProviderEndpoint {
  id: string
  provider_id: string
  format: 'completions' | 'responses' | 'anthropic' | 'gemini'
  path: string
}

export interface ApiKeyInfo {
  id: string
  label: string
  is_active: boolean
  usage_count: number
  last_used_at: string | null
  created_at: string
}

export interface ModelRoute {
  id: string
  model_pattern: string
  alias: string | null
  provider_id: string
  target_model: string
  target_format: string
  fallback_provider_id: string | null
  priority: number
}

export interface RequestLog {
  id: number
  request_id: string
  client_format: string
  provider_name: string
  provider_format: string
  model: string
  stream: boolean
  status_code: number
  duration_ms: number
  prompt_tokens: number
  completion_tokens: number
  total_tokens: number
  error_message: string | null
  created_at: string
}

export interface UsageSummary {
  model: string
  provider_name: string
  total_prompt_tokens: number
  total_completion_tokens: number
  total_tokens: number
  total_cost: number
  request_count: number
}

export interface InterceptorRule {
  id: string
  name: string
  phase: 'pre' | 'post'
  condition: RuleCondition
  action: RuleAction
  priority: number
  enabled: boolean
}

export type RuleCondition =
  | { type: 'model_matches'; pattern: string }
  | { type: 'path_contains'; substring: string }
  | { type: 'header_exists'; name: string }
  | { type: 'always' }

export type RuleAction =
  | { type: 'replace_model'; new_model: string }
  | { type: 'set_header'; name: string; value: string }
  | { type: 'remove_header'; name: string }
  | { type: 'inject_system_prompt'; text: string }
  | { type: 'override_parameter'; key: string; value: unknown }
  | { type: 'filter_response'; pattern: string; replacement: string }
