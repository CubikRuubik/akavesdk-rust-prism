import { AkaveWebSDK } from '@akave/akave-web-sdk';

declare global {
    interface Window {
        ethereum?: {
            request: (args: { method: string; params?: any[] }) => Promise<any>;
            isMetaMask?: boolean;
            isCoinbaseWallet?: boolean;
            isWalletConnect?: boolean;
        };
    }
}

export interface Bucket {
    name: string;
}

export interface File {
    name: string;
}

export interface AppState {
    sdk: AkaveWebSDK | null;
    currentAddress: string | null;
}

export interface Notification {
    message: string;
    type: 'error' | 'success';
}
