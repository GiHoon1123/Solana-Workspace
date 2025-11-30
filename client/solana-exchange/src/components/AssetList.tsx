'use client';

import { useEffect, useState } from 'react';
import { apiClient, AssetPosition } from '@/lib/api';

export default function AssetList() {
  const [positions, setPositions] = useState<AssetPosition[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [isMounted, setIsMounted] = useState(false);

  // Hydration 에러 방지: 클라이언트에서만 렌더링
  useEffect(() => {
    setIsMounted(true);
  }, []);

  useEffect(() => {
    if (!isMounted) return;

    const fetchPositions = async () => {
      if (!apiClient.isAuthenticated()) {
        setLoading(false);
        return;
      }

      try {
        setLoading(true);
        setError(null);
        
        // 먼저 positions API 시도, 실패하면 balances API 사용
        try {
          const response = await apiClient.getPositions();
          
          // USDT를 최상단에 고정
          const sortedPositions = response.positions.sort((a, b) => {
            if (a.mint === 'USDT') return -1;
            if (b.mint === 'USDT') return 1;
            return a.mint.localeCompare(b.mint); // 나머지는 알파벳 순
          });
          
          setPositions(sortedPositions);
        } catch (positionsError) {
          // positions API가 404면 balances API로 폴백
          if (positionsError instanceof Error && positionsError.message.includes('404')) {
            console.log('Positions API를 사용할 수 없어 Balances API로 전환합니다.');
            const balancesResponse = await apiClient.getBalances();
            
            // balances를 positions 형식으로 변환 (USDT 포함)
            const convertedPositions: AssetPosition[] = balancesResponse.balances
              .map(b => ({
                mint: b.mint_address,
                current_balance: (parseFloat(b.available) + parseFloat(b.locked)).toString(),
                available: b.available,
                locked: b.locked,
                average_entry_price: null,
                total_bought_amount: '0',
                total_bought_cost: '0',
                // USDT는 가격이 항상 1 USDT = 1 USDT
                current_market_price: b.mint_address === 'USDT' ? '1' : null,
                // USDT는 평가액 = 잔액 (1:1)
                current_value: b.mint_address === 'USDT' 
                  ? (parseFloat(b.available) + parseFloat(b.locked)).toString()
                  : null,
                unrealized_pnl: null,
                unrealized_pnl_percent: null,
                trade_summary: {
                  total_buy_trades: 0,
                  total_sell_trades: 0,
                  realized_pnl: '0',
                },
              }));
            
            // USDT를 최상단에 고정
            const sortedPositions = convertedPositions.sort((a, b) => {
              if (a.mint === 'USDT') return -1;
              if (b.mint === 'USDT') return 1;
              return a.mint.localeCompare(b.mint); // 나머지는 알파벳 순
            });
            
            setPositions(sortedPositions);
          } else {
            throw positionsError;
          }
        }
      } catch (err) {
        console.error('자산 내역 로딩 실패:', err);
        let errorMessage = '자산 내역 로딩 실패';
        
        if (err instanceof Error) {
          errorMessage = err.message;
          
          // 401 에러는 인증 문제
          if (err.message.includes('401') || err.message.includes('Unauthorized')) {
            errorMessage = '인증이 필요합니다. 다시 로그인해주세요.';
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

    fetchPositions();

    // 5초마다 자산 내역 갱신
    const interval = setInterval(fetchPositions, 5000);

    return () => clearInterval(interval);
  }, [isMounted]);

  const formatNumber = (value: string | null | undefined, decimals: number = 2): string => {
    if (!value) return '--';
    const num = parseFloat(value);
    if (isNaN(num)) return '--';
    return num.toFixed(decimals);
  };

  const formatCurrency = (value: string | null | undefined): string => {
    if (!value) return '--';
    const num = parseFloat(value);
    if (isNaN(num)) return '--';
    return `$${num.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`;
  };

  // Hydration 에러 방지: 클라이언트 마운트 전에는 빈 div 반환
  if (!isMounted) {
    return (
      <div className="w-64 bg-gray-800 border-r border-gray-700 flex flex-col h-full">
        <div className="p-4 border-b border-gray-700 flex-shrink-0">
          <h3 className="text-base font-semibold text-white">자산 내역</h3>
        </div>
      </div>
    );
  }

  if (!apiClient.isAuthenticated()) {
    return (
      <div className="w-64 bg-gray-800 border-r border-gray-700 p-4 flex items-center justify-center">
        <p className="text-gray-400 text-sm">로그인이 필요합니다</p>
      </div>
    );
  }

  if (loading) {
    return (
      <div className="w-64 bg-gray-800 border-r border-gray-700 flex flex-col h-full">
        <div className="p-4 border-b border-gray-700 flex-shrink-0">
          <h3 className="text-base font-semibold text-white">자산 내역</h3>
        </div>
        <div className="text-gray-400 text-center py-4 text-sm">로딩 중...</div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="w-64 bg-gray-800 border-r border-gray-700 flex flex-col h-full">
        <div className="p-4 border-b border-gray-700 flex-shrink-0">
          <h3 className="text-base font-semibold text-white">자산 내역</h3>
        </div>
        <div className="text-red-400 text-center py-4 text-sm px-4">{error}</div>
      </div>
    );
  }

  return (
    <div className="w-64 bg-gray-800 border-r border-gray-700 flex flex-col h-full">
      <div className="p-4 border-b border-gray-700 flex-shrink-0">
        <h3 className="text-base font-semibold text-white">자산 내역</h3>
      </div>
      
      <div className="flex-1 overflow-y-auto">
        {positions.length === 0 ? (
          <div className="p-4 text-gray-400 text-center text-sm">
            보유한 자산이 없습니다
          </div>
        ) : (
          <div className="p-2">
            {positions.map((position) => {
              const pnl = position.unrealized_pnl ? parseFloat(position.unrealized_pnl) : 0;
              const pnlPercent = position.unrealized_pnl_percent ? parseFloat(position.unrealized_pnl_percent) : 0;
              const isProfit = pnl >= 0;

              return (
                <div
                  key={position.mint}
                  className="p-3 mb-2 bg-gray-900 rounded-lg border border-gray-700 hover:border-gray-600 transition-colors"
                >
                  {/* 자산명 */}
                  <div className="flex items-center justify-between mb-2">
                    <span className="text-white font-semibold text-sm">{position.mint}</span>
                    {position.current_market_price && (
                      <span className="text-gray-400 text-xs">
                        ${formatNumber(position.current_market_price)}
                      </span>
                    )}
                  </div>

                  {/* 보유 수량 */}
                  <div className="mb-2">
                    <div className="flex items-center justify-between text-xs mb-1">
                      <span className="text-gray-400">보유</span>
                      <span className="text-white font-medium">
                        {formatNumber(position.current_balance, 4)} {position.mint}
                      </span>
                    </div>
                    {parseFloat(position.locked) > 0 && (
                      <div className="flex items-center justify-between text-xs">
                        <span className="text-gray-500">잠김</span>
                        <span className="text-gray-500">
                          {formatNumber(position.locked, 4)} {position.mint}
                        </span>
                      </div>
                    )}
                  </div>

                  {/* 평가액 */}
                  {position.current_value && (
                    <div className="mb-2">
                      <div className="flex items-center justify-between text-xs">
                        <span className="text-gray-400">평가액</span>
                        <span className="text-white font-medium">
                          {formatCurrency(position.current_value)}
                        </span>
                      </div>
                    </div>
                  )}

                  {/* 평균 매수가 */}
                  {position.average_entry_price && (
                    <div className="mb-2">
                      <div className="flex items-center justify-between text-xs">
                        <span className="text-gray-400">평균 매수가</span>
                        <span className="text-gray-300">
                          ${formatNumber(position.average_entry_price)}
                        </span>
                      </div>
                    </div>
                  )}

                  {/* 손익 */}
                  {position.unrealized_pnl !== null && position.unrealized_pnl_percent !== null && (
                    <div className="pt-2 border-t border-gray-700">
                      <div className="flex items-center justify-between text-xs">
                        <span className="text-gray-400">손익</span>
                        <div className="flex items-center gap-2">
                          <span className={`font-semibold ${isProfit ? 'text-red-400' : 'text-blue-400'}`}>
                            {isProfit ? '+' : ''}{formatCurrency(position.unrealized_pnl)}
                          </span>
                          <span className={`font-semibold ${isProfit ? 'text-red-400' : 'text-blue-400'}`}>
                            ({isProfit ? '+' : ''}{formatNumber(position.unrealized_pnl_percent)}%)
                          </span>
                        </div>
                      </div>
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}

