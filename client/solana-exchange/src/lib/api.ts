const API_BASE_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:3002';

// 토큰 저장소 키
const ACCESS_TOKEN_KEY = 'access_token';
const REFRESH_TOKEN_KEY = 'refresh_token';

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
  refresh_token: string;
  message: string;
}

export interface RefreshTokenRequest {
  refresh_token: string;
}

export interface RefreshTokenResponse {
  access_token: string;
  refresh_token: string;
  message: string;
}

export interface LogoutRequest {
  refresh_token: string;
}

// 토큰 저장소 관리
class TokenStorage {
  static getAccessToken(): string | null {
    if (typeof window === 'undefined') return null;
    return localStorage.getItem(ACCESS_TOKEN_KEY);
  }

  static getRefreshToken(): string | null {
    if (typeof window === 'undefined') return null;
    return localStorage.getItem(REFRESH_TOKEN_KEY);
  }

  static setTokens(accessToken: string, refreshToken: string): void {
    if (typeof window === 'undefined') return;
    localStorage.setItem(ACCESS_TOKEN_KEY, accessToken);
    localStorage.setItem(REFRESH_TOKEN_KEY, refreshToken);
  }

  static clearTokens(): void {
    if (typeof window === 'undefined') return;
    localStorage.removeItem(ACCESS_TOKEN_KEY);
    localStorage.removeItem(REFRESH_TOKEN_KEY);
  }

  static hasTokens(): boolean {
    return !!this.getAccessToken() && !!this.getRefreshToken();
  }
}

// API 클라이언트
class ApiClient {
  private baseUrl: string;
  private isRefreshing: boolean = false;
  private refreshPromise: Promise<void> | null = null;

  constructor(baseUrl: string) {
    this.baseUrl = baseUrl;
  }

  // 토큰 갱신
  private async refreshAccessToken(): Promise<void> {
    const refreshToken = TokenStorage.getRefreshToken();
    if (!refreshToken) {
      throw new Error('No refresh token available');
    }

    try {
      const response = await fetch(`${this.baseUrl}/api/auth/refresh`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ refresh_token: refreshToken }),
      });

      if (!response.ok) {
        // Refresh 실패 시 로그아웃 처리
        TokenStorage.clearTokens();
        throw new Error('Token refresh failed');
      }

      const data: RefreshTokenResponse = await response.json();
      TokenStorage.setTokens(data.access_token, data.refresh_token);
    } catch (error) {
      TokenStorage.clearTokens();
      throw error;
    }
  }

  private async request<T>(
    endpoint: string,
    options: RequestInit = {},
    retry: boolean = true
  ): Promise<T> {
    const url = `${this.baseUrl}${endpoint}`;
    
    // Access Token 자동 추가 (인증이 필요한 요청)
    const accessToken = TokenStorage.getAccessToken();
    const headers: HeadersInit = {
      'Content-Type': 'application/json',
      ...options.headers,
    };

    if (accessToken && !endpoint.includes('/auth/')) {
      headers['Authorization'] = `Bearer ${accessToken}`;
    }

    const response = await fetch(url, {
      ...options,
      headers,
    });

    // 401 에러 시 토큰 갱신 시도
    if (response.status === 401 && retry && accessToken) {
      // 이미 갱신 중이면 대기
      if (this.isRefreshing && this.refreshPromise) {
        await this.refreshPromise;
        // 갱신 후 재시도
        return this.request<T>(endpoint, options, false);
      }

      // 토큰 갱신 시작
      this.isRefreshing = true;
      this.refreshPromise = this.refreshAccessToken();

      try {
        await this.refreshPromise;
        // 갱신 성공 후 재시도
        return this.request<T>(endpoint, options, false);
      } catch (error) {
        // 갱신 실패 시 에러 발생
        throw new Error('Authentication failed. Please login again.');
      } finally {
        this.isRefreshing = false;
        this.refreshPromise = null;
      }
    }

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
    const response = await this.request<SigninResponse>('/api/auth/signin', {
      method: 'POST',
      body: JSON.stringify(data),
    });
    
    // 토큰 저장
    TokenStorage.setTokens(response.access_token, response.refresh_token);
    
    return response;
  }

  // 토큰 갱신
  async refresh(): Promise<RefreshTokenResponse> {
    const refreshToken = TokenStorage.getRefreshToken();
    if (!refreshToken) {
      throw new Error('No refresh token available');
    }

    const response = await this.request<RefreshTokenResponse>('/api/auth/refresh', {
      method: 'POST',
      body: JSON.stringify({ refresh_token: refreshToken }),
    });

    // 새 토큰 저장
    TokenStorage.setTokens(response.access_token, response.refresh_token);

    return response;
  }

  // 로그아웃
  async logout(): Promise<void> {
    const refreshToken = TokenStorage.getRefreshToken();
    
    if (refreshToken) {
      try {
        await this.request('/api/auth/logout', {
          method: 'POST',
          body: JSON.stringify({ refresh_token: refreshToken }),
        });
      } catch (error) {
        // 로그아웃 실패해도 클라이언트에서는 토큰 제거
        console.error('Logout request failed:', error);
      }
    }

    // 클라이언트 토큰 제거
    TokenStorage.clearTokens();
  }

  // 인증 상태 확인
  isAuthenticated(): boolean {
    return TokenStorage.hasTokens();
  }
}

export const apiClient = new ApiClient(API_BASE_URL);
export { TokenStorage };

