import init, { AkaveWebSDKBuilder } from "@akave/akave-rs";
import { AppState, Bucket, File, Notification } from "./types";
import "./styles.css";

// Enable WASM logging
const originalConsoleLog = console.log;
console.log = (...args) => {
  originalConsoleLog.apply(console, args);
  // Log WASM messages if they exist
  if (args[0] && typeof args[0] === "string" && args[0].includes("WASM")) {
    originalConsoleLog("[WASM]", ...args);
  }
};

class App {
  private state: AppState = {
    sdk: null,
    currentAddress: null,
  };

  // DOM Elements
  private connectWalletBtn: HTMLButtonElement;
  private walletAddressSpan: HTMLSpanElement;
  private bucketNameInput: HTMLInputElement;
  private createBucketBtn: HTMLButtonElement;
  private bucketsList: HTMLDivElement;
  private bucketSelect: HTMLSelectElement;
  private fileInput: HTMLInputElement;
  private uploadFileBtn: HTMLButtonElement;
  private filesList: HTMLDivElement;

  constructor() {
    // Initialize DOM elements
    this.connectWalletBtn = document.getElementById(
      "connectWallet",
    ) as HTMLButtonElement;
    this.walletAddressSpan = document.getElementById(
      "walletAddress",
    ) as HTMLSpanElement;
    this.bucketNameInput = document.getElementById(
      "bucketName",
    ) as HTMLInputElement;
    this.createBucketBtn = document.getElementById(
      "createBucket",
    ) as HTMLButtonElement;
    this.bucketsList = document.getElementById("bucketsList") as HTMLDivElement;
    this.bucketSelect = document.getElementById(
      "bucketSelect",
    ) as HTMLSelectElement;
    this.fileInput = document.getElementById("fileInput") as HTMLInputElement;
    this.uploadFileBtn = document.getElementById(
      "uploadFile",
    ) as HTMLButtonElement;
    this.filesList = document.getElementById("filesList") as HTMLDivElement;

    // Bind event listeners
    this.connectWalletBtn.addEventListener("click", () => this.connectWallet());
    this.createBucketBtn.addEventListener("click", () => this.createBucket());
    this.bucketSelect.addEventListener("change", (e) =>
      this.handleBucketSelect(e),
    );
    this.uploadFileBtn.addEventListener("click", () => this.uploadFile());

    // Initialize the application
    this.initialize();
  }

  private async initialize(): Promise<void> {
    try {
      console.log("Starting SDK initialization...");
      // Initialize the WASM module
      await init();
      console.log("WASM module initialized");

      // Create SDK instance
      console.log("Creating SDK instance...");
      const sdk = await new AkaveWebSDKBuilder("http://23.227.172.82:7001/grpc")
        .withDefaultEncryption("testkey123")
        .withErasureCoding(4, 2)
        .build();
      this.state.sdk = sdk;
      console.log("SDK instance created successfully");

      this.connectWalletBtn.disabled = false;
    } catch (error) {
      console.error("Failed to initialize:", error);
      this.showNotification({
        message:
          "Failed to initialize SDK. Please check the console for details.",
        type: "error",
      });
    }
  }

  private async connectWallet(): Promise<void> {
    try {
      console.log("Starting wallet connection process...");
      if (!window.ethereum) {
        this.showNotification({
          message:
            "No Ethereum wallet found. Please install MetaMask or another compatible wallet.",
          type: "error",
        });
        return;
      }

      console.log("Requesting account access...");
      const accounts = await window.ethereum.request({
        method: "eth_requestAccounts",
      });
      console.log(accounts);
      if (!accounts || !accounts[0]) {
        throw new Error("No accounts returned from wallet");
      }

      const address = accounts[0];
      console.log("Wallet connected successfully:", address);
      this.state.currentAddress = address;

      if (!this.walletAddressSpan || !this.connectWalletBtn) {
        throw new Error("Required DOM elements not found");
      }

      this.walletAddressSpan.textContent = `${address.slice(
        0,
        6,
      )}...${address.slice(-4)}`;
      this.connectWalletBtn.textContent = "Connected";
      this.connectWalletBtn.disabled = true;

      this.showNotification({
        message: `Wallet connected: ${address.slice(0, 6)}...${address.slice(
          -4,
        )}`,
        type: "success",
      });

      console.log("Loading buckets for connected wallet...");
      await this.loadBuckets();
    } catch (error) {
      console.error("Failed to connect wallet:", error);
      this.showNotification({
        message:
          "Failed to connect wallet. Please make sure your wallet is unlocked.",
        type: "error",
      });
    }
  }

  private async createBucket(): Promise<void> {
    try {
      console.log("Starting bucket creation process...");
      const bucketName = this.bucketNameInput.value.trim();
      if (!bucketName) {
        this.showNotification({
          message: "Please enter a bucket name",
          type: "error",
        });
        return;
      }

      if (!this.state.sdk) {
        throw new Error("SDK not initialized");
      }

      console.log(`Creating bucket: ${bucketName}`);
      await this.state.sdk.createBucket(bucketName);
      console.log(`Bucket "${bucketName}" created successfully`);

      this.showNotification({
        message: `Bucket "${bucketName}" created successfully!`,
        type: "success",
      });

      this.bucketNameInput.value = "";
      console.log("Reloading buckets list...");
      await this.loadBuckets();
    } catch (error) {
      console.error("Failed to create bucket:", error);
      this.showNotification({
        message:
          "Failed to create bucket. Please check the console for details.",
        type: "error",
      });
    }
  }

  private async loadBuckets(): Promise<void> {
    try {
      if (!this.state.currentAddress || !this.state.sdk) return;

      console.log(`Fetching buckets for address: ${this.state.currentAddress}`);
      const response = await this.state.sdk.listBuckets();
      console.log(`Found ${response.buckets.length} buckets`, response.buckets);

      this.bucketsList.innerHTML = "";
      this.bucketSelect.innerHTML = '<option value="">Select a bucket</option>';

      response.buckets.forEach((bucket: Bucket) => {
        console.log(`Processing bucket: ${bucket.name}`);
        // Add to buckets list
        const bucketElement = document.createElement("div");
        bucketElement.className = "list-item";
        bucketElement.innerHTML = `
                    <span>${bucket.name}</span>
                    <button class="btn" data-bucket="${bucket.name}">Delete</button>
                `;
        bucketElement
          .querySelector("button")
          ?.addEventListener("click", () => this.deleteBucket(bucket.name));
        this.bucketsList.appendChild(bucketElement);

        // Add to select dropdown
        const option = document.createElement("option");
        option.value = bucket.name;
        option.textContent = bucket.name;
        this.bucketSelect.appendChild(option);
      });

      this.bucketSelect.disabled = false;
    } catch (error) {
      console.error("Failed to load buckets:", error);
      this.showNotification({
        message:
          "Failed to load buckets. Please check the console for details.",
        type: "error",
      });
    }
  }

  private async deleteBucket(bucketName: string): Promise<void> {
    try {
      if (!this.state.currentAddress || !this.state.sdk) return;

      console.log(`Deleting bucket: ${bucketName}`);
      await this.state.sdk.deleteBucket(bucketName);
      console.log(`Bucket "${bucketName}" deleted successfully`);

      this.showNotification({
        message: `Bucket "${bucketName}" deleted successfully!`,
        type: "success",
      });

      console.log("Reloading buckets list...");
      await this.loadBuckets();
    } catch (error) {
      console.error("Failed to delete bucket:", error);
      this.showNotification({
        message:
          "Failed to delete bucket. Please check the console for details.",
        type: "error",
      });
    }
  }

  private async loadFiles(bucketName: string): Promise<void> {
    try {
      if (!this.state.currentAddress || !this.state.sdk) return;

      console.log(`Fetching files for bucket: ${bucketName}`);
      const response = await this.state.sdk.listFiles(bucketName);
      console.log(
        `Found ${response.files.length} files in bucket ${bucketName}`,
      );

      this.filesList.innerHTML = "";

      response.files.forEach((file: File) => {
        console.log(`Processing file: ${file.name}`);
        const fileElement = document.createElement("div");
        fileElement.className = "list-item";
        fileElement.innerHTML = `
                    <span>${file.name}</span>
                    <div class="button-group">
                        <button class="btn download-btn" data-file="${file.name}">Download</button>
                        <button class="btn delete-btn" data-file="${file.name}">Delete</button>
                    </div>
                `;
        // Add event listeners for buttons
        fileElement
          .querySelector(".download-btn")
          ?.addEventListener("click", () =>
            this.downloadFile(bucketName, file.name),
          );
        fileElement
          .querySelector(".delete-btn")
          ?.addEventListener("click", () =>
            this.deleteFile(bucketName, file.name),
          );
        this.filesList.appendChild(fileElement);
      });
    } catch (error) {
      console.error("Failed to load files:", error);
      this.showNotification({
        message: "Failed to load files. Please check the console for details.",
        type: "error",
      });
    }
  }

  private async downloadFile(
    bucketName: string,
    fileName: string,
  ): Promise<void> {
    try {
      if (!this.state.currentAddress || !this.state.sdk) return;

      console.log(`Downloading file: ${fileName} from bucket: ${bucketName}`);

      // Show loading notification
      this.showNotification({
        message: `Downloading ${fileName}...`,
        type: "info",
      });

      // Create a temporary client-side path for the download
      // In browser context, this is handled by the SDK internally
      const tempPath = "download"; // The actual path is handled by the WASM implementation

      // Trigger download from SDK
      const result = await this.state.sdk.downloadFile(bucketName, fileName);

      console.log(`File "${fileName}" download initiated successfully`);

      if (result) {
        const blob = new Blob([result]);
        const url = URL.createObjectURL(blob);
        const a = document.createElement("a");
        a.href = url;
        a.download = "file.txt";
        a.click();
        URL.revokeObjectURL(url);

        this.showNotification({
          message: `File "${fileName}" downloaded successfully!`,
          type: "success",
        });
      } else {
        console.error("Download result is empty");
        this.showNotification({
          message: "Failed to download file. No data received.",
          type: "error",
        });
      }
    } catch (error) {
      console.error("Failed to download file:", error);
      this.showNotification({
        message:
          "Failed to download file. Please check the console for details.",
        type: "error",
      });
    }
  }

  private async deleteFile(
    bucketName: string,
    fileName: string,
  ): Promise<void> {
    try {
      if (!this.state.currentAddress || !this.state.sdk) return;

      console.log(`Deleting file: ${fileName}`);
      await this.state.sdk.deleteFile(bucketName, fileName);
      console.log(`File "${fileName}" deleted successfully`);

      this.showNotification({
        message: `File "${fileName}" deleted successfully!`,
        type: "success",
      });

      console.log("Reloading files list...");
      await this.loadFiles(bucketName);
    } catch (error) {
      console.error("Failed to delete file:", error);
      this.showNotification({
        message: "Failed to delete file. Please check the console for details.",
        type: "error",
      });
    }
  }

  private async uploadFile(): Promise<void> {
    try {
      const bucketName = this.bucketSelect.value;
      const file = this.fileInput.files?.[0];

      if (!bucketName || !file || !this.state.sdk) {
        this.showNotification({
          message: "Please select both a bucket and a file",
          type: "error",
        });
        return;
      }

      console.log(`Uploading file: ${file.name} to bucket: ${bucketName}`);
      const arrayBuffer = await file.arrayBuffer();
      await this.state.sdk.uploadFile(
        bucketName,
        file.name,
        new Uint8Array(arrayBuffer),
      );
      console.log(`File "${file.name}" uploaded successfully`);

      this.showNotification({
        message: `File "${file.name}" uploaded successfully!`,
        type: "success",
      });

      console.log("Reloading files list...");
      await this.loadFiles(bucketName);
    } catch (error) {
      console.error("Failed to upload file:", error);
      this.showNotification({
        message: "Failed to upload file. Please check the console for details.",
        type: "error",
      });
    }
  }

  private handleBucketSelect(event: Event): void {
    const select = event.target as HTMLSelectElement;
    if (select.value) {
      console.log(`Bucket selected: ${select.value}`);
      this.loadFiles(select.value);
      this.uploadFileBtn.disabled = false;
    } else {
      console.log("No bucket selected");
      this.uploadFileBtn.disabled = true;
      this.filesList.innerHTML = "";
    }
  }

  private showNotification(notification: Notification): void {
    const notificationElement = document.createElement("div");
    notificationElement.className = notification.type;
    notificationElement.textContent = notification.message;
    document.body.appendChild(notificationElement);
    setTimeout(() => notificationElement.remove(), 5000);
  }
}

// Initialize the application when the DOM is loaded
document.addEventListener("DOMContentLoaded", () => {
  new App();
});
