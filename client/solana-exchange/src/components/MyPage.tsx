'use client';

import { useState, useEffect } from 'react';
import { apiClient, WalletsResponse, BalanceResponse, CreateWalletResponse } from '@/lib/api';

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

export default function MyPage({ onClose }: { onClose: () => void }) {
  const [wallet, setWallet] = useState<Wallet | null>(null);
  const [balance, setBalance] = useState<WalletBalance | null>(null);
  const [loading, setLoading] = useState(true);
  const [creating, setCreating] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // 로그인 시 저장된 유저 정보 (임시)
  const [userInfo, setUserInfo] = useState<{
    email: string | null;
    username: string | null;
  }>({ email: null, username: null });

  useEffect(() => {
    // localStorage에서 유저 정보 가져오기 (로그인 시 저장된 정보)
    const storedUser = localStorage.getItem('user_info');
    if (storedUser) {
      try {
        const user = JSON.parse(storedUser);
        setUserInfo({
          email: user.email || null,
          username: user.username || null,
        });
      } catch (e) {
        console.error('Failed to parse user info:', e);
      }
    }

    fetchWalletData();
  }, []);

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
    } catch (err) {
      console.error('지갑 생성 실패:', err);
      setError(err instanceof Error ? err.message : '지갑 생성에 실패했습니다.');
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
      <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
        <div className="bg-gray-800 rounded-lg p-6 w-full max-w-2xl max-h-[90vh] overflow-y-auto">
          <div className="text-gray-400 text-center py-8">로딩 중...</div>
        </div>
      </div>
    );
  }

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
      <div className="bg-gray-800 rounded-lg p-6 w-full max-w-2xl max-h-[90vh] overflow-y-auto">
        <div className="flex justify-between items-center mb-6">
          <h2 className="text-2xl font-bold text-white">마이페이지</h2>
          <button
            onClick={onClose}
            className="text-gray-400 hover:text-white text-2xl"
          >
            ×
          </button>
        </div>

        {/* 내 정보 */}
        <div className="mb-6">
          <h3 className="text-lg font-semibold text-white mb-4">내 정보</h3>
          <div className="bg-gray-900 rounded-lg p-4 space-y-3">
            <div>
              <label className="text-sm text-gray-400">이메일</label>
              <div className="text-white mt-1">
                {userInfo.email || '정보 없음'}
              </div>
            </div>
            <div>
              <label className="text-sm text-gray-400">사용자명</label>
              <div className="text-white mt-1">
                {userInfo.username || '정보 없음'}
              </div>
            </div>
          </div>
        </div>

        {/* 지갑 관리 */}
        <div className="mb-6">
          <h3 className="text-lg font-semibold text-white mb-4">지갑 관리</h3>
          {wallet ? (
            <div className="bg-gray-900 rounded-lg p-4 space-y-4">
              <div>
                <label className="text-sm text-gray-400">Public Key</label>
                <div className="flex items-center gap-2 mt-2">
                  <div className="flex-1 bg-gray-800 rounded px-3 py-2 text-white text-sm break-all">
                    {wallet.public_key}
                  </div>
                  <button
                    onClick={() => copyToClipboard(wallet.public_key)}
                    className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded text-sm whitespace-nowrap"
                  >
                    복사
                  </button>
                </div>
              </div>
              {balance && (
                <div>
                  <label className="text-sm text-gray-400">SOL 잔액</label>
                  <div className="text-white text-xl font-semibold mt-1">
                    {balance.balance_sol.toFixed(4)} SOL
                  </div>
                  <div className="text-gray-400 text-sm mt-1">
                    ≈ ${(balance.balance_sol * 145.23).toFixed(2)} USD
                  </div>
                </div>
              )}
              <div>
                <label className="text-sm text-gray-400">생성일</label>
                <div className="text-white mt-1">
                  {new Date(wallet.created_at).toLocaleString('ko-KR')}
                </div>
              </div>
            </div>
          ) : (
            <div className="bg-gray-900 rounded-lg p-6 text-center">
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
        {wallet && balance && (
          <div>
            <h3 className="text-lg font-semibold text-white mb-4">자산 내역</h3>
            <div className="bg-gray-900 rounded-lg p-4">
              <div className="mb-3">
                <label className="text-sm text-gray-400">총 보유 자산</label>
                <div className="text-white text-2xl font-bold mt-1">
                  {balance.balance_sol.toFixed(4)} SOL
                </div>
                <div className="text-gray-400 text-sm mt-1">
                  ≈ ${(balance.balance_sol * 145.23).toFixed(2)} USD
                </div>
              </div>
            </div>
          </div>
        )}

        {error && (
          <div className="mt-4 bg-red-900 bg-opacity-50 border border-red-700 text-red-200 px-4 py-3 rounded">
            {error}
          </div>
        )}
      </div>
    </div>
  );
}

