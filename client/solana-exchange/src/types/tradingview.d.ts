// TradingView 위젯 타입 정의
declare global {
  interface Window {
    TradingView: {
      widget: new (options: TradingViewWidgetOptions) => TradingViewWidget;
    };
  }
}

interface TradingViewWidgetOptions {
  autosize?: boolean;
  symbol: string;
  interval?: string;
  timezone?: string;
  theme?: 'light' | 'dark';
  style?: string;
  locale?: string;
  toolbar_bg?: string;
  enable_publishing?: boolean;
  hide_top_toolbar?: boolean;
  hide_legend?: boolean;
  save_image?: boolean;
  container_id: string;
  studies?: string[];
  colors?: {
    [key: string]: string;
  };
}

interface TradingViewWidget {
  // 위젯 메서드들
}

export {};

