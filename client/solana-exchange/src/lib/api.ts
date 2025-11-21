const API_BASE_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:3002';

// API 타입 정의
export interface SignupRequest {
  email: string;
  password: string;
  username?: string;
}

export interface SigninRequest {
  email: string;
  password: string;
}

export interface UserResponse {
  id: number;
  email: string;
  username: string | null;
  created_at: string;
  updated_at: string;
}

export interface SignupResponse {
  user: UserResponse;
  message: string;
}

export interface SigninResponse {
  user: UserResponse;
  access_token: string;
  message: string;
}

// API 클라이언트
class ApiClient {
  private baseUrl: string;

  constructor(baseUrl: string) {
    this.baseUrl = baseUrl;
  }

  private async request<T>(
    endpoint: string,
    options: RequestInit = {}
  ): Promise<T> {
    const url = `${this.baseUrl}${endpoint}`;
    const headers = {
      'Content-Type': 'application/json',
      ...options.headers,
    };

    const response = await fetch(url, {
      ...options,
      headers,
    });

    if (!response.ok) {
      const error = await response.json().catch(() => ({ error: 'Unknown error' }));
      throw new Error(error.error || `HTTP error! status: ${response.status}`);
    }

    return response.json();
  }

  // 회원가입
  async signup(data: SignupRequest): Promise<SignupResponse> {
    return this.request<SignupResponse>('/api/auth/signup', {
      method: 'POST',
      body: JSON.stringify(data),
    });
  }

  // 로그인
  async signin(data: SigninRequest): Promise<SigninResponse> {
    return this.request<SigninResponse>('/api/auth/signin', {
      method: 'POST',
      body: JSON.stringify(data),
    });
  }
}

export const apiClient = new ApiClient(API_BASE_URL);

