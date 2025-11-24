'use client';

import { useEffect, useRef, useState } from 'react';

/**
 * TradingView 솔라나 차트 위젯
 * 업비트와 동일한 차트 제공
 */
export default function SolanaChart() {
  const containerRef = useRef<HTMLDivElement>(null);
  const [containerId, setContainerId] = useState<string>('');
  const [isMounted, setIsMounted] = useState(false);

  // 클라이언트에서만 ID 생성 (하이드레이션 에러 방지)
  useEffect(() => {
    setIsMounted(true);
    setContainerId(`tradingview_${Math.random().toString(36).substr(2, 9)}`);
  }, []);

  useEffect(() => {
    if (!containerRef.current || !containerId || !isMounted) return;

    // 이미 스크립트가 로드되어 있는지 확인
    if (window.TradingView) {
      initializeChart();
      return;
    }

    // TradingView 위젯 스크립트 로드
    const script = document.createElement('script');
    script.src = 'https://s3.tradingview.com/tv.js';
    script.async = true;
    script.onload = initializeChart;

    document.head.appendChild(script);

    return () => {
      // 클린업
      if (document.head.contains(script)) {
        document.head.removeChild(script);
      }
    };
  }, [containerId, isMounted]);

  const initializeChart = () => {
    if (!containerRef.current || !window.TradingView || !containerId) return;

    new window.TradingView.widget({
      autosize: true,
      symbol: 'BINANCE:SOLUSDT', // 바이낸스 SOL/USDT 차트
      interval: 'D', // 일봉 (1, 5, 15, 60, D, W, M 등 가능)
      timezone: 'Asia/Seoul',
      theme: 'dark', // 다크 테마
      style: '1', // 캔들스틱 스타일
      locale: 'kr', // 한국어
      toolbar_bg: '#1a1a1a',
      enable_publishing: false,
      hide_top_toolbar: false,
      hide_legend: false,
      save_image: false,
      container_id: containerId,
      // 차트 설정
      studies: [
        'Volume@tv-basicstudies',
        'RSI@tv-basicstudies',
      ],
      // 색상 커스터마이징
      colors: {
        'paneProperties.background': '#1a1a1a',
        'paneProperties.vertGridProperties.color': '#2a2a2a',
        'paneProperties.horzGridProperties.color': '#2a2a2a',
      },
    });
  };

  return (
    <div className="w-full h-full bg-gray-900 rounded-lg overflow-hidden border border-gray-700">
      <div
        id={containerId || undefined}
        ref={containerRef}
        className="w-full h-full"
      />
    </div>
  );
}

