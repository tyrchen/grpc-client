export interface ServerStatus {
  id: string;
  name: string;
  endpoint: string;
  connected: boolean;
  description?: string;
  lastConnected?: string;
  error?: string;
}

export interface ServiceInfo {
  name: string;
  description?: string;
  methods: MethodInfo[];
}

export interface MethodInfo {
  name: string;
  inputType: string;
  outputType: string;
  clientStreaming: boolean;
  serverStreaming: boolean;
  streamingType: string;
  description?: string;
}

export interface CallRequest {
  method: string;
  data: any;
  headers?: Record<string, string>;
  emitDefaults?: boolean;
}

export interface CallResponse {
  success: boolean;
  response: any[];
  error?: string;
}

export interface MethodSchema {
  server_id: string;
  service_name: string;
  method_name: string;
  input_type: string;
  output_type: string;
  schema: any;
  validation_rules: any;
  streaming_type: string;
}

export interface ErrorResponse {
  error: string;
  details?: string;
}

export interface CallHistoryEntry {
  id: string;
  timestamp: string;
  serverId: string;
  serviceName: string;
  methodName: string;
  request: any;
  response: CallResponse;
  duration: number;
  headers?: Record<string, string>;
}

class ApiClient {
  private baseUrl: string;

  constructor(baseUrl: string = 'http://localhost:4000') {
    this.baseUrl = baseUrl;
  }

  private async request<T>(
    endpoint: string,
    options: RequestInit = {}
  ): Promise<T> {
    const url = `${this.baseUrl}${endpoint}`;

    try {
      const response = await fetch(url, {
        headers: {
          'Content-Type': 'application/json',
          ...options.headers,
        },
        ...options,
      });

      if (!response.ok) {
        const errorData: ErrorResponse = await response.json().catch(() => ({
          error: `HTTP ${response.status}: ${response.statusText}`,
        }));
        throw new Error(errorData.details || errorData.error);
      }

      return await response.json();
    } catch (error) {
      if (error instanceof Error) {
        throw error;
      }
      throw new Error('Network error occurred');
    }
  }

  // Health check
  async health(): Promise<{ status: string; service: string; timestamp: string }> {
    return this.request('/api/health');
  }

  // Server operations
  async listServers(): Promise<ServerStatus[]> {
    return this.request('/api/servers');
  }

  // Service operations
  async listServices(serverId: string): Promise<{ server_id: string; services: any[] }> {
    return this.request(`/api/servers/${encodeURIComponent(serverId)}/services`);
  }

  async describeService(
    serverId: string,
    serviceName: string
  ): Promise<{ server_id: string; service_name: string; description: any }> {
    return this.request(
      `/api/servers/${encodeURIComponent(serverId)}/services/${encodeURIComponent(serviceName)}`
    );
  }

  // Method operations
  async getMethodSchema(
    serverId: string,
    serviceName: string,
    methodName: string
  ): Promise<MethodSchema> {
    return this.request(
      `/api/servers/${encodeURIComponent(serverId)}/services/${encodeURIComponent(serviceName)}/methods/${encodeURIComponent(methodName)}`
    );
  }

  async callMethod(serverId: string, request: CallRequest): Promise<CallResponse> {
    return this.request(`/api/servers/${encodeURIComponent(serverId)}/call`, {
      method: 'POST',
      body: JSON.stringify(request),
    });
  }
}

export const apiClient = new ApiClient();
