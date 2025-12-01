'use client';

import { useState, useEffect } from 'react';
import { apiClient, Trade, Order } from '@/lib/api';

export default function MyTrades() {
  const [trades, setTrades] = useState<Trade[]>([]);
  const [myOrderIds, setMyOrderIds] = useState<Set<number>>(new Set());
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchTrades = async () => {
    if (!apiClient.isAuthenticated()) {
      setLoading(false);
      return;
    }

    try {
      setError(null);
      
      // 1. 내 주문 목록 가져오기 (주문 ID 수집용)
      const myOrders = await apiClient.getMyOrders(undefined, 1000, 0);
      const orderIdSet = new Set(myOrders.map(order => order.id));
      setMyOrderIds(orderIdSet);
      
      // 2. 내 체결 내역 가져오기
      const myTrades = await apiClient.getMyTrades('SOL', 50, 0);
      // 최신순 정렬
      const sortedTrades = myTrades.sort((a, b) => 
        new Date(b.created_at).getTime() - new Date(a.created_at).getTime()
      );
      setTrades(sortedTrades);
    } catch (err) {
      console.error('체결 내역 가져오기 실패:', err);
      setError(err instanceof Error ? err.message : '체결 내역을 불러올 수 없습니다.');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchTrades();
    // 3초마다 체결 내역 갱신
    const interval = setInterval(fetchTrades, 3000);
    return () => clearInterval(interval);
  }, []);

  const formatDate = (dateString: string) => {
    const date = new Date(dateString);
    return `${date.getHours().toString().padStart(2, '0')}:${date.getMinutes().toString().padStart(2, '0')}:${date.getSeconds().toString().padStart(2, '0')}`;
  };

  if (loading) {
    return (
      <div className="h-full flex flex-col">
        <h3 className="text-base font-semibold text-white mb-4">내 체결</h3>
        <div className="text-gray-400 text-center py-4">로딩 중...</div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="h-full flex flex-col">
        <h3 className="text-base font-semibold text-white mb-4">내 체결</h3>
        <div className="text-red-400 text-center py-4 text-sm">{error}</div>
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col overflow-hidden">
      <h3 className="text-base font-semibold text-white mb-4 flex-shrink-0">내 체결</h3>
      
      <div className="flex-1 overflow-y-auto">
        {trades.length === 0 ? (
          <div className="text-gray-400 text-center py-8 text-sm">체결 내역이 없습니다.</div>
        ) : (
          <div className="space-y-2">
            {trades.map((trade) => {
              const totalValue = parseFloat(trade.price) * parseFloat(trade.amount);
              
              // 매수/매도 구분: 내 주문 ID가 buy_order_id에 있으면 매수, sell_order_id에 있으면 매도
              const isBuy = myOrderIds.has(trade.buy_order_id);
              const isSell = myOrderIds.has(trade.sell_order_id);
              const tradeType = isBuy ? 'buy' : isSell ? 'sell' : 'unknown';
              
              return (
                <div
                  key={trade.id}
                  className="bg-gray-900 rounded-lg border border-gray-700 p-3 hover:border-gray-600 transition-colors"
                >
                  <div className="flex items-center justify-between mb-2">
                    <div className="flex items-center gap-2">
                      <span className={`text-xs font-semibold ${
                        tradeType === 'buy' ? 'text-red-400' : tradeType === 'sell' ? 'text-blue-400' : 'text-gray-400'
                      }`}>
                        {tradeType === 'buy' ? '매수' : tradeType === 'sell' ? '매도' : '체결'}
                      </span>
                      <span className="text-xs font-semibold text-white">
                        {parseFloat(trade.amount).toFixed(4)} SOL
                      </span>
                    </div>
                    <span className="text-xs text-gray-400">{formatDate(trade.created_at)}</span>
                  </div>

                  <div className="grid grid-cols-2 gap-2 text-xs">
                    <div>
                      <span className="text-gray-400">가격:</span>
                      <span className="text-white ml-2">
                        ${parseFloat(trade.price).toFixed(2)}
                      </span>
                    </div>
                    <div>
                      <span className="text-gray-400">총액:</span>
                      <span className="text-white ml-2">
                        ${totalValue.toFixed(2)}
                      </span>
                    </div>
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
    );
}

