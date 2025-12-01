'use client';

import { useState, useEffect } from 'react';
import { apiClient, UserResponse, Balance } from '@/lib/api';
import { useRouter } from 'next/navigation';
import { useAlert } from '@/components/AlertModal';

interface Wallet {
  id: number;
  user_id: number;
  public_key: string;
  encrypted_private_key: string;
  created_at: string;
  updated_at: string;
}

interface WalletBalance {
  balance_lamports: number;
  balance_sol: number;
  public_key: string;
}

export default function MyPagePage() {
  const router = useRouter();
  const [wallet, setWallet] = useState<Wallet | null>(null);
  const [balance, setBalance] = useState<WalletBalance | null>(null);
  const [loading, setLoading] = useState(true);
  const [creating, setCreating] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [user, setUser] = useState<UserResponse | null>(null);
  const [exchangeBalances, setExchangeBalances] = useState<Balance[]>([]);
  const { showAlert, AlertContainer } = useAlert();

  useEffect(() => {
    const fetchMyPageData = async () => {
      if (!apiClient.isAuthenticated()) {
        router.push('/');
        return;
      }

      try {
        // 1. 유저 정보 가져오기
        const userData = await apiClient.getMe();
        setUser(userData);

        // 2. 지갑 정보 가져오기
        await fetchWalletData();

        // 3. 거래소 잔고 가져오기
        const balancesResponse = await apiClient.getBalances();
        setExchangeBalances(balancesResponse.balances);
      } catch (err) {
        console.error('마이페이지 데이터 로딩 실패:', err);
        let errorMessage = err instanceof Error ? err.message : '데이터 로딩 실패';
        
        if (err instanceof Error) {
          if (err.message.includes('401') || err.message.includes('Unauthorized') || err.message.includes('Missing authorization header')) {
            errorMessage = '인증이 필요합니다. 다시 로그인해주세요.';
            apiClient.logout();
            router.push('/');
          } else if (err.message.includes('500')) {
            errorMessage = '서버 오류가 발생했습니다. 잠시 후 다시 시도해주세요.';
          } else if (err.message.includes('Failed to fetch') || err.message.includes('NetworkError')) {
            errorMessage = '네트워크 오류가 발생했습니다. 연결을 확인해주세요.';
          }
        }
        setError(errorMessage);
      } finally {
        setLoading(false);
      }
    };

    fetchMyPageData();
  }, [router]);

  const fetchWalletData = async () => {
    if (!apiClient.isAuthenticated()) {
      setLoading(false);
      return;
    }

    try {
      setError(null);
      // 내 지갑 목록 조회
      const walletsResponse = await apiClient.getUserWallets();
      
      if (walletsResponse.wallets && walletsResponse.wallets.length > 0) {
        // 1:1 관계이므로 첫 번째 지갑만 사용
        const myWallet = walletsResponse.wallets[0];
        setWallet(myWallet);

        // 지갑 잔액 조회
        const balanceResponse = await apiClient.getWalletBalance(myWallet.id);
        setBalance(balanceResponse);
      }
    } catch (err) {
      console.error('지갑 정보 조회 실패:', err);
      setError(err instanceof Error ? err.message : '지갑 정보를 불러올 수 없습니다.');
    } finally {
      setLoading(false);
    }
  };

  const handleCreateWallet = async () => {
    if (!apiClient.isAuthenticated()) {
      setError('로그인이 필요합니다.');
      return;
    }

    setCreating(true);
    setError(null);

    try {
      await apiClient.createWallet();
      // 지갑 생성 후 다시 조회
      await fetchWalletData();
      showAlert('지갑이 성공적으로 생성되었습니다.', 'success');
    } catch (err) {
      console.error('지갑 생성 실패:', err);
      const errorMessage = err instanceof Error ? err.message : '지갑 생성에 실패했습니다.';
      setError(errorMessage);
      showAlert(errorMessage, 'error');
    } finally {
      setCreating(false);
    }
  };

  const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text);
    alert('복사되었습니다!');
  };

  if (loading) {
    return (
      <main className="min-h-screen bg-gray-900">
        <div className="max-w-[1920px] mx-auto px-6 py-8">
          <div className="text-gray-400 text-center py-8">로딩 중...</div>
        </div>
      </main>
    );
  }

  return (
    <main className="min-h-screen bg-gray-900">
      <div className="max-w-[1920px] mx-auto px-6 py-8">
        <h1 className="text-3xl font-bold text-white mb-8">마이페이지</h1>

        {/* 내 정보 */}
        <div className="mb-8">
          <h2 className="text-xl font-semibold text-white mb-4">내 정보</h2>
          <div className="bg-gray-800 rounded-lg border border-gray-700 p-6 space-y-4">
            <div>
              <label className="text-sm text-gray-400">이메일</label>
              <div className="text-white mt-1">
                {user?.email || '정보 없음'}
              </div>
            </div>
            <div>
              <label className="text-sm text-gray-400">사용자명</label>
              <div className="text-white mt-1">
                {user?.username || '정보 없음'}
              </div>
            </div>
            {user?.created_at && (
              <div>
                <label className="text-sm text-gray-400">가입일</label>
                <div className="text-white mt-1">
                  {new Date(user.created_at).toLocaleDateString()}
                </div>
              </div>
            )}
          </div>
        </div>

        {/* 지갑 관리 */}
        <div className="mb-8">
          <h2 className="text-xl font-semibold text-white mb-4">지갑 관리</h2>
          {wallet ? (
            <div className="bg-gray-800 rounded-lg border border-gray-700 p-6 space-y-4">
              <div>
                <label className="text-sm text-gray-400">Public Key</label>
                <div className="flex items-center gap-2 mt-2">
                  <div className="flex-1 bg-gray-900 rounded px-4 py-3 text-white text-sm break-all border border-gray-700">
                    {wallet.public_key}
                  </div>
                  <button
                    onClick={() => copyToClipboard(wallet.public_key)}
                    className="px-4 py-3 bg-blue-600 hover:bg-blue-700 text-white rounded text-sm whitespace-nowrap transition-colors"
                  >
                    복사
                  </button>
                </div>
              </div>
              <div>
                <label className="text-sm text-gray-400">생성일</label>
                <div className="text-white mt-1">
                  {new Date(wallet.created_at).toLocaleString('ko-KR')}
                </div>
              </div>
            </div>
          ) : (
            <div className="bg-gray-800 rounded-lg border border-gray-700 p-8 text-center">
              <p className="text-gray-400 mb-4">생성된 지갑이 없습니다.</p>
              <button
                onClick={handleCreateWallet}
                disabled={creating}
                className="px-6 py-3 bg-blue-600 hover:bg-blue-700 text-white rounded font-semibold transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
              >
                {creating ? '생성 중...' : '지갑 생성하기'}
              </button>
            </div>
          )}
        </div>

        {/* 자산 내역 */}
        <div className="mb-8">
          <h2 className="text-xl font-semibold text-white mb-4">자산 내역</h2>
          <div className="bg-gray-800 rounded-lg border border-gray-700 p-6 space-y-4">
            {exchangeBalances.length === 0 ? (
              <div className="text-gray-400 text-center py-4">보유한 자산이 없습니다.</div>
            ) : (
              exchangeBalances
                .filter(b => b.mint_address === 'SOL' || b.mint_address === 'USDT')
                .sort((a, b) => {
                  // USDT를 먼저, 그 다음 SOL
                  if (a.mint_address === 'USDT') return -1;
                  if (b.mint_address === 'USDT') return 1;
                  return 0;
                })
                .map((balance) => {
                  const totalBalance = parseFloat(balance.available) + parseFloat(balance.locked);
                  return (
                    <div key={balance.mint_address} className="border-b border-gray-700 pb-4 last:border-0 last:pb-0">
                      <div className="flex items-center justify-between mb-2">
                        <span className="text-white font-semibold">{balance.mint_address}</span>
                        <span className="text-white font-medium">
                          {totalBalance.toFixed(balance.mint_address === 'USDT' ? 2 : 4)} {balance.mint_address}
                        </span>
                      </div>
                      <div className="flex items-center justify-between text-sm">
                        <span className="text-gray-400">사용 가능</span>
                        <span className="text-gray-300">
                          {parseFloat(balance.available).toFixed(balance.mint_address === 'USDT' ? 2 : 4)} {balance.mint_address}
                        </span>
                      </div>
                      {parseFloat(balance.locked) > 0 && (
                        <div className="flex items-center justify-between text-sm mt-1">
                          <span className="text-gray-400">잠김</span>
                          <span className="text-gray-300">
                            {parseFloat(balance.locked).toFixed(balance.mint_address === 'USDT' ? 2 : 4)} {balance.mint_address}
                          </span>
                        </div>
                      )}
                    </div>
                  );
                })
            )}
          </div>
        </div>

        {error && (
          <div className="mt-6 bg-red-900 bg-opacity-50 border border-red-700 text-red-200 px-4 py-3 rounded">
            {error}
          </div>
        )}
      </div>
      <AlertContainer />
    </main>
  );
}

