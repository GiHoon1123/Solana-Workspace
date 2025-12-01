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

    // 인증이 필요한 엔드포인트에 Authorization 헤더 추가
    // /api/auth/signup, /api/auth/signin, /api/auth/refresh, /api/auth/logout은 제외
    if (accessToken && !endpoint.includes('/auth/signup') && !endpoint.includes('/auth/signin') && !endpoint.includes('/auth/refresh') && !endpoint.includes('/auth/logout')) {
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
      let errorMessage = `HTTP error! status: ${response.status}`;
      try {
        const error = await response.json();
        errorMessage = error.error || error.message || errorMessage;
      } catch {
        // JSON 파싱 실패 시 기본 메시지 사용
        errorMessage = `HTTP error! status: ${response.status}`;
      }
      throw new Error(errorMessage);
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

  // 유저 정보 조회
  async getMe(): Promise<UserResponse> {
    return this.request<UserResponse>('/api/auth/me', {
      method: 'GET',
    });
  }

  // 지갑 관련 API
  async getUserWallets(): Promise<WalletsResponse> {
    return this.request<WalletsResponse>('/api/wallets/my', {
      method: 'GET',
    });
  }

  async createWallet(): Promise<CreateWalletResponse> {
    return this.request<CreateWalletResponse>('/api/wallets', {
      method: 'POST',
    });
  }

  async getWalletBalance(walletId: number): Promise<BalanceResponse> {
    return this.request<BalanceResponse>(`/api/wallets/${walletId}/balance`, {
      method: 'GET',
    });
  }

  // CEX Positions API
  async getPositions(): Promise<AllPositionsResponse> {
    return this.request<AllPositionsResponse>('/api/cex/positions', {
      method: 'GET',
    });
  }

  // CEX Balances API (폴백용)
  async getBalances(): Promise<{ balances: Balance[] }> {
    return this.request<{ balances: Balance[] }>('/api/cex/balances', {
      method: 'GET',
    });
  }

  async getPosition(mint: string): Promise<AssetPositionResponse> {
    return this.request<AssetPositionResponse>(`/api/cex/positions/${mint}`, {
      method: 'GET',
    });
  }

  // CEX Orders API
  async createOrder(request: CreateOrderRequest): Promise<Order> {
    return this.request<Order>('/api/cex/orders', {
      method: 'POST',
      body: JSON.stringify(request),
    });
  }

  async getMyOrders(status?: string, limit?: number, offset?: number): Promise<Order[]> {
    const params = new URLSearchParams();
    if (status) params.append('status', status);
    if (limit) params.append('limit', limit.toString());
    if (offset) params.append('offset', offset.toString());
    
    const queryString = params.toString();
    const endpoint = `/api/cex/orders/my${queryString ? `?${queryString}` : ''}`;
    
    return this.request<Order[]>(endpoint, {
      method: 'GET',
    });
  }

  async cancelOrder(orderId: number): Promise<Order> {
    return this.request<Order>(`/api/cex/orders/${orderId}`, {
      method: 'DELETE',
    });
  }

  async getOrder(orderId: number): Promise<Order> {
    return this.request<Order>(`/api/cex/orders/${orderId}`, {
      method: 'GET',
    });
  }

  // CEX Trades API
  async getMyTrades(mint?: string, limit?: number, offset?: number): Promise<Trade[]> {
    const params = new URLSearchParams();
    if (mint) params.append('mint', mint);
    if (limit) params.append('limit', limit.toString());
    if (offset) params.append('offset', offset.toString());
    
    const queryString = params.toString();
    const endpoint = `/api/cex/trades/my${queryString ? `?${queryString}` : ''}`;
    
    return this.request<Trade[]>(endpoint, {
      method: 'GET',
    });
  }
}

// 지갑 관련 타입
export interface SolanaWallet {
  id: number;
  user_id: number;
  public_key: string;
  encrypted_private_key: string;
  created_at: string;
  updated_at: string;
}

export interface WalletsResponse {
  wallets: SolanaWallet[];
}

export interface CreateWalletResponse {
  wallet: SolanaWallet;
  message: string;
}

export interface BalanceResponse {
  balance_lamports: number;
  balance_sol: number;
  public_key: string;
}

// CEX Positions 타입
export interface AssetPosition {
  mint: string;
  current_balance: string;
  available: string;
  locked: string;
  average_entry_price: string | null;
  total_bought_amount: string;
  total_bought_cost: string;
  current_market_price: string | null;
  current_value: string | null;
  unrealized_pnl: string | null;
  unrealized_pnl_percent: string | null;
  trade_summary: {
    total_buy_trades: number;
    total_sell_trades: number;
    realized_pnl: string;
  };
}

export interface AllPositionsResponse {
  positions: AssetPosition[];
}

export interface AssetPositionResponse {
  position: AssetPosition;
}

// CEX Balances 타입
export interface Balance {
  id: number;
  user_id: number;
  mint_address: string;
  available: string;
  locked: string;
  created_at: string;
  updated_at: string;
}

// CEX Orders 타입
export interface CreateOrderRequest {
  order_type: 'buy' | 'sell';
  order_side: 'limit' | 'market';
  base_mint: string;
  quote_mint?: string;
  price?: string; // 지정가 주문만
  amount?: string; // 지정가 매수, 모든 매도
  quote_amount?: string; // 시장가 매수만
}

export interface Order {
  id: number;
  user_id: number;
  order_type: string;
  order_side: string;
  base_mint: string;
  quote_mint: string;
  price: string | null;
  amount: string;
  filled_amount: string;
  filled_quote_amount: string;
  status: 'pending' | 'partial' | 'filled' | 'cancelled';
  created_at: string;
  updated_at: string;
}

// CEX Trades 타입
export interface Trade {
  id: number;
  buy_order_id: number;
  sell_order_id: number;
  base_mint: string;
  quote_mint: string;
  price: string;
  amount: string;
  created_at: string;
}

export const apiClient = new ApiClient(API_BASE_URL);
export { TokenStorage };

