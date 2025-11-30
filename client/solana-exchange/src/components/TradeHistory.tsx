'use client';

import { useEffect, useState, useRef } from 'react';

interface Trade {
  time: string;
  price: number;
  amount: number;
  type: 'buy' | 'sell';
}

export default function TradeHistory() {
  const [trades, setTrades] = useState<Trade[]>([]);
  const [loading, setLoading] = useState(true);
  const wsRef = useRef<WebSocket | null>(null);

  useEffect(() => {
    // 초기 체결 내역 가져오기
    const fetchInitialTrades = async () => {
      try {
        const response = await fetch(
          'https://api.binance.com/api/v3/trades?symbol=SOLUSDT&limit=20'
        );
        const data = await response.json();

        const formattedTrades: Trade[] = data
          .reverse()
          .map((trade: any) => {
            const date = new Date(trade.time);
            const time = `${date.getHours().toString().padStart(2, '0')}:${date
              .getMinutes()
              .toString()
              .padStart(2, '0')}:${date.getSeconds().toString().padStart(2, '0')}`;

            const price = parseFloat(trade.price) || 0;
            const amount = parseFloat(trade.qty) || 0;

            return {
              time,
              price,
              amount,
              type: trade.isBuyerMaker ? 'sell' : 'buy',
            };
          })
          .filter((t: Trade) => t.price > 0 && t.amount > 0);

        setTrades(formattedTrades);
        setLoading(false);
      } catch (error) {
        console.error('초기 체결 내역 가져오기 실패:', error);
        setLoading(false);
      }
    };

    fetchInitialTrades();

    // 실시간 체결 내역 WebSocket
    wsRef.current = new WebSocket('wss://stream.binance.com:9443/ws/solusdt@trade');

    wsRef.current.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);
        const date = new Date(data.T);
        const time = `${date.getHours().toString().padStart(2, '0')}:${date
          .getMinutes()
          .toString()
          .padStart(2, '0')}:${date.getSeconds().toString().padStart(2, '0')}`;

        const price = parseFloat(data.p) || 0;
        const amount = parseFloat(data.q) || 0;

        if (price > 0 && amount > 0) {
          const newTrade: Trade = {
            time,
            price,
            amount,
            type: data.m ? 'sell' : 'buy', // m이 true면 매도
          };

          setTrades((prev) => {
            const updated = [newTrade, ...prev].slice(0, 20);
            return updated;
          });
        }
      } catch (error) {
        console.error('WebSocket 데이터 파싱 실패:', error);
      }
    };

    wsRef.current.onerror = (error) => {
      // WebSocket 에러는 조용히 처리 (재연결 시도하지 않음)
      console.warn('체결 내역 WebSocket 연결 오류 (자동 재연결 시도)');
    };

    wsRef.current.onclose = () => {
      // 연결이 끊어지면 자동으로 재연결 시도
      console.warn('체결 내역 WebSocket 연결 종료');
    };

    return () => {
      if (wsRef.current) {
        wsRef.current.close();
      }
    };
  }, []);

  if (loading) {
    return (
      <div className="bg-gray-800 rounded-lg border border-gray-700 p-5 h-full flex flex-col">
        <h3 className="text-base font-semibold text-white mb-4">체결 내역</h3>
        <div className="text-gray-400 text-center py-4">로딩 중...</div>
      </div>
    );
  }

  return (
    <div className="bg-gray-800 rounded-lg border border-gray-700 p-5 h-full flex flex-col">
      <h3 className="text-base font-semibold text-white mb-4">체결 내역</h3>
      <div className="space-y-0.5 overflow-y-auto">
        {trades.length === 0 ? (
          <div className="text-gray-400 text-center py-4">체결 내역이 없습니다.</div>
        ) : (
          trades.map((trade, index) => (
            <div
              key={`trade-${index}`}
              className="flex justify-between items-center text-xs hover:bg-gray-700 px-2 py-1 rounded"
            >
              <span className="text-gray-400 text-xs">{trade.time}</span>
              <span
                className={`font-medium ${
                  trade.type === 'buy' ? 'text-red-400' : 'text-blue-400'
                }`}
              >
                {trade.price > 0 ? trade.price.toFixed(2) : '--'}
              </span>
              <span className="text-gray-300">
                {trade.amount > 0 ? trade.amount.toFixed(4) : '--'}
              </span>
            </div>
          ))
        )}
      </div>
    </div>
  );
}

