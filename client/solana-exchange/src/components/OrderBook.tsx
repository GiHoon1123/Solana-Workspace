'use client';

import { useEffect, useState, useRef } from 'react';

interface OrderBookEntry {
  price: number;
  amount: number;
  total: number;
}

export default function OrderBook() {
  const [buyOrders, setBuyOrders] = useState<OrderBookEntry[]>([]);
  const [sellOrders, setSellOrders] = useState<OrderBookEntry[]>([]);
  const [currentPrice, setCurrentPrice] = useState<number>(0);
  const [priceChange, setPriceChange] = useState<number>(0);
  const [loading, setLoading] = useState(true);
  const wsDepthRef = useRef<WebSocket | null>(null);
  const wsTickerRef = useRef<WebSocket | null>(null);
  const depthSnapshotRef = useRef<{ bids: string[][]; asks: string[][] } | null>(null);

  useEffect(() => {
    // 초기 스냅샷 가져오기
    const fetchSnapshot = async () => {
      try {
        const response = await fetch(
          'https://api.binance.com/api/v3/depth?symbol=SOLUSDT&limit=20'
        );
        const data = await response.json();
        depthSnapshotRef.current = data;

        // 초기 데이터 설정
        const asks = data.asks
          .slice(0, 10)
          .map(([price, quantity]: string[]) => {
            const p = parseFloat(price) || 0;
            const q = parseFloat(quantity) || 0;
            return {
              price: p,
              amount: q,
              total: p * q,
            };
          })
          .filter((o: OrderBookEntry) => o.price > 0 && o.amount > 0)
          .sort((a: OrderBookEntry, b: OrderBookEntry) => a.price - b.price);

        const bids = data.bids
          .slice(0, 10)
          .map(([price, quantity]: string[]) => {
            const p = parseFloat(price) || 0;
            const q = parseFloat(quantity) || 0;
            return {
              price: p,
              amount: q,
              total: p * q,
            };
          })
          .filter((o: OrderBookEntry) => o.price > 0 && o.amount > 0)
          .sort((a: OrderBookEntry, b: OrderBookEntry) => b.price - a.price);

        setSellOrders(asks);
        setBuyOrders(bids);
        setLoading(false);
      } catch (error) {
        console.error('스냅샷 가져오기 실패:', error);
        setLoading(false);
      }
    };

    fetchSnapshot();

    // 호가창 WebSocket (depth stream)
    wsDepthRef.current = new WebSocket('wss://stream.binance.com:9443/ws/solusdt@depth20@100ms');

    wsDepthRef.current.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);
        
        if (data.bids && data.asks) {
          const asks = data.asks
            .map(([price, quantity]: string[]) => {
              const p = parseFloat(price) || 0;
              const q = parseFloat(quantity) || 0;
              return { price: p, amount: q, total: p * q };
            })
            .filter((o: OrderBookEntry) => o.price > 0 && o.amount > 0)
            .sort((a: OrderBookEntry, b: OrderBookEntry) => a.price - b.price)
            .slice(0, 10);

          const bids = data.bids
            .map(([price, quantity]: string[]) => {
              const p = parseFloat(price) || 0;
              const q = parseFloat(quantity) || 0;
              return { price: p, amount: q, total: p * q };
            })
            .filter((o: OrderBookEntry) => o.price > 0 && o.amount > 0)
            .sort((a: OrderBookEntry, b: OrderBookEntry) => b.price - a.price)
            .slice(0, 10);

          setSellOrders(asks);
          setBuyOrders(bids);
        }
      } catch (error) {
        console.error('WebSocket 데이터 파싱 실패:', error);
      }
    };

    wsDepthRef.current.onerror = (error) => {
      // WebSocket 에러는 조용히 처리 (재연결 시도하지 않음)
      // 네트워크 문제나 일시적 오류는 자동으로 재연결됨
      console.warn('호가창 WebSocket 연결 오류 (자동 재연결 시도)');
    };

    wsDepthRef.current.onclose = () => {
      // 연결이 끊어지면 자동으로 재연결 시도
      console.warn('호가창 WebSocket 연결 종료');
    };

    // 현재가 WebSocket (ticker stream)
    wsTickerRef.current = new WebSocket('wss://stream.binance.com:9443/ws/solusdt@ticker');

    wsTickerRef.current.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);
        const price = parseFloat(data.c) || 0; // 현재가
        const change = parseFloat(data.P) || 0; // 24시간 변동률

        if (price > 0) {
          setCurrentPrice(price);
          setPriceChange(change);
        }
      } catch (error) {
        console.error('Ticker 데이터 파싱 실패:', error);
      }
    };

    wsTickerRef.current.onerror = (error) => {
      // WebSocket 에러는 조용히 처리 (재연결 시도하지 않음)
      console.warn('현재가 WebSocket 연결 오류 (자동 재연결 시도)');
    };

    wsTickerRef.current.onclose = () => {
      // 연결이 끊어지면 자동으로 재연결 시도
      console.warn('현재가 WebSocket 연결 종료');
    };

    return () => {
      if (wsDepthRef.current) {
        wsDepthRef.current.close();
      }
      if (wsTickerRef.current) {
        wsTickerRef.current.close();
      }
    };
  }, []);

  if (loading) {
    return (
      <div className="bg-gray-800 rounded-lg border border-gray-700 p-5 h-full flex flex-col">
        <h3 className="text-base font-semibold text-white mb-5">호가창</h3>
        <div className="text-gray-400 text-center py-4">로딩 중...</div>
      </div>
    );
  }

  return (
    <div className="bg-gray-800 rounded-lg border border-gray-700 p-5 h-full flex flex-col overflow-hidden">
      <h3 className="text-base font-semibold text-white mb-4 flex-shrink-0">호가창</h3>
      
      <div className="flex-1 flex flex-col min-h-0 overflow-hidden">
        {/* 매도 호가 */}
        <div className="flex-1 min-h-0 flex flex-col">
          <div className="text-xs text-blue-400 font-semibold mb-2 flex-shrink-0">매도</div>
          <div className="flex-1 overflow-y-auto space-y-0.5">
            {sellOrders.map((order, index) => (
              <div
                key={`sell-${index}`}
                className="flex justify-between items-center text-sm hover:bg-gray-700 px-2 py-1 rounded"
              >
                <span className="text-blue-400">
                  {order.price > 0 ? order.price.toFixed(2) : '--'}
                </span>
                <span className="text-gray-300">
                  {order.amount > 0 ? order.amount.toFixed(4) : '--'}
                </span>
                <span className="text-gray-400">
                  {order.total > 0 ? order.total.toFixed(2) : '--'}
                </span>
              </div>
            ))}
          </div>
        </div>

        {/* 현재가 */}
        <div className="text-center py-3 border-y border-gray-700 my-3 flex-shrink-0">
          <div className="text-xl font-bold text-blue-400">
            {currentPrice > 0 ? currentPrice.toFixed(2) : '--'}
          </div>
          <div
            className={`text-xs mt-1 ${
              priceChange >= 0 ? 'text-red-400' : 'text-blue-400'
            }`}
          >
            {currentPrice > 0 && (
              <>
                {priceChange >= 0 ? '+' : ''}
                {priceChange.toFixed(2)}%
              </>
            )}
          </div>
        </div>

        {/* 매수 호가 */}
        <div className="flex-1 min-h-0 flex flex-col">
          <div className="text-xs text-red-400 font-semibold mb-2 flex-shrink-0">매수</div>
          <div className="flex-1 overflow-y-auto space-y-0.5">
            {buyOrders.map((order, index) => (
              <div
                key={`buy-${index}`}
                className="flex justify-between items-center text-sm hover:bg-gray-700 px-2 py-1 rounded"
              >
                <span className="text-red-400">
                  {order.price > 0 ? order.price.toFixed(2) : '--'}
                </span>
                <span className="text-gray-300">
                  {order.amount > 0 ? order.amount.toFixed(4) : '--'}
                </span>
                <span className="text-gray-400">
                  {order.total > 0 ? order.total.toFixed(2) : '--'}
                </span>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}

