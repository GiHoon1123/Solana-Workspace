'use client';

import { useEffect, useState } from 'react';
import { apiClient, AssetPosition } from '@/lib/api';

export default function AssetList() {
  const [positions, setPositions] = useState<AssetPosition[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [isMounted, setIsMounted] = useState(false);
  const [solPrice, setSolPrice] = useState<number | null>(null); // SOL 현재 가격 (USDT)

  // Hydration 에러 방지: 클라이언트에서만 렌더링
  useEffect(() => {
    setIsMounted(true);
  }, []);

  // SOL 현재 가격 가져오기 (바이낸스 WebSocket - 실시간)
  useEffect(() => {
    // 초기 가격 가져오기 (REST API)
    const fetchInitialPrice = async () => {
      try {
        const response = await fetch('https://api.binance.com/api/v3/ticker/price?symbol=SOLUSDT');
        const data = await response.json();
        const price = parseFloat(data.price) || null;
        if (price) setSolPrice(price);
      } catch (error) {
        console.error('SOL 초기 가격 가져오기 실패:', error);
      }
    };

    fetchInitialPrice();

    // WebSocket으로 실시간 가격 받기
    const ws = new WebSocket('wss://stream.binance.com:9443/ws/solusdt@ticker');

    ws.onopen = () => {
      console.log('AssetList: SOL 가격 WebSocket 연결됨');
    };

    ws.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);
        const price = parseFloat(data.c) || null; // 현재가 (last price)
        if (price && price > 0) {
          setSolPrice(price);
        }
      } catch (error) {
        console.error('SOL 가격 WebSocket 데이터 파싱 실패:', error);
      }
    };

    ws.onerror = (error) => {
      console.warn('AssetList: SOL 가격 WebSocket 연결 오류');
    };

    ws.onclose = () => {
      console.warn('AssetList: SOL 가격 WebSocket 연결 종료');
    };

    return () => {
      ws.close();
    };
  }, []);

  // 자산 내역 가져오기 (초기 로드 및 주기적 갱신)
  useEffect(() => {
    if (!isMounted) return;

    const fetchPositions = async () => {
      if (!apiClient.isAuthenticated()) {
        setLoading(false);
        return;
      }

      try {
        // solPrice 변경 시에는 로딩 상태를 표시하지 않음 (깜빡임 방지)
        if (positions.length === 0) {
          setLoading(true);
        }
        setError(null);
        
        // balances API를 우선 사용 (USDT 포함 보장)
        const balancesResponse = await apiClient.getBalances();
        
        // positions API에서 추가 정보 가져오기 (손익 등)
        let positionsData: AssetPosition[] = [];
        try {
          const positionsResponse = await apiClient.getPositions();
          positionsData = positionsResponse.positions;
        } catch (positionsError) {
          // positions API 실패해도 계속 진행 (balances만 사용)
          console.log('Positions API를 사용할 수 없어 Balances만 사용합니다.');
        }
        
        // balances를 positions 형식으로 변환하고, positions API 데이터와 병합
        const convertedPositions: AssetPosition[] = balancesResponse.balances
          .map(b => {
            const balance = parseFloat(b.available) + parseFloat(b.locked);
            
            // positions API에서 해당 mint의 데이터 찾기
            const positionData = positionsData.find(p => p.mint === b.mint_address);
            
            // SOL의 경우 바이낸스 가격 사용, USDT는 1, 그 외는 positions API 가격 사용
            let marketPrice: string | null = null;
            let value: string | null = null;
            
            if (b.mint_address === 'USDT') {
              marketPrice = '1';
              value = balance.toString();
            } else if (b.mint_address === 'SOL' && solPrice) {
              marketPrice = solPrice.toString();
              value = (solPrice * balance).toString();
            } else if (positionData?.current_market_price) {
              marketPrice = positionData.current_market_price;
              value = positionData.current_value || null;
            }
            
            return {
              mint: b.mint_address,
              current_balance: balance.toString(),
              available: b.available,
              locked: b.locked,
              average_entry_price: positionData?.average_entry_price || null,
              total_bought_amount: positionData?.total_bought_amount || '0',
              total_bought_cost: positionData?.total_bought_cost || '0',
              current_market_price: marketPrice,
              current_value: value,
              unrealized_pnl: positionData?.unrealized_pnl || null,
              unrealized_pnl_percent: positionData?.unrealized_pnl_percent || null,
              trade_summary: positionData?.trade_summary || {
                total_buy_trades: 0,
                total_sell_trades: 0,
                realized_pnl: '0',
              },
            };
          });
        
        // USDT를 최상단에 고정
        const sortedPositions = convertedPositions.sort((a, b) => {
          if (a.mint === 'USDT') return -1;
          if (b.mint === 'USDT') return 1;
          return a.mint.localeCompare(b.mint); // 나머지는 알파벳 순
        });
        
        setPositions(sortedPositions);
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
  }, [isMounted]); // solPrice 제거 - 가격 변경 시에는 positions를 다시 가져오지 않음

  // solPrice 변경 시 기존 positions의 평가액만 업데이트 (깜빡임 방지)
  useEffect(() => {
    if (!solPrice || positions.length === 0) return;

    setPositions(prevPositions => 
      prevPositions.map(position => {
        // SOL의 경우만 가격과 평가액 업데이트
        if (position.mint === 'SOL') {
          const balance = parseFloat(position.current_balance);
          return {
            ...position,
            current_market_price: solPrice.toString(),
            current_value: (solPrice * balance).toString(),
          };
        }
        return position;
      })
    );
  }, [solPrice]); // solPrice만 의존 - positions는 의존성에서 제외

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
                    {(() => {
                      // SOL의 경우 바이낸스 가격 사용, 그 외는 백엔드 가격 사용
                      const price = position.mint === 'SOL' && solPrice 
                        ? solPrice 
                        : position.current_market_price 
                          ? parseFloat(position.current_market_price) 
                          : null;
                      
                      return price ? (
                        <span className="text-gray-400 text-xs">
                          ${formatNumber(price.toString())}
                        </span>
                      ) : null;
                    })()}
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
                  {(() => {
                    // 평가액 계산: 현재 가격 × 보유 수량
                    let calculatedValue: number | null = null;
                    
                    if (position.mint === 'SOL' && solPrice) {
                      // SOL의 경우 바이낸스 가격 사용
                      calculatedValue = solPrice * parseFloat(position.current_balance);
                    } else if (position.current_value) {
                      // 백엔드에서 제공하는 평가액 사용
                      calculatedValue = parseFloat(position.current_value);
                    } else if (position.current_market_price) {
                      // 백엔드 가격으로 계산
                      calculatedValue = parseFloat(position.current_market_price) * parseFloat(position.current_balance);
                    } else if (position.mint === 'USDT') {
                      // USDT는 1:1
                      calculatedValue = parseFloat(position.current_balance);
                    }
                    
                    return calculatedValue !== null ? (
                      <div className="mb-2">
                        <div className="flex items-center justify-between text-xs">
                          <span className="text-gray-400">평가액</span>
                          <span className="text-white font-medium">
                            {formatCurrency(calculatedValue.toString())}
                          </span>
                        </div>
                      </div>
                    ) : null;
                  })()}

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

