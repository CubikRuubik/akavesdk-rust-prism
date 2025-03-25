import init, { AkaveWebSDK } from '../../pkg/akave_wasm_sdk.js';

// DOM Elements
const connectWalletBtn = document.getElementById('connectWallet');
const walletAddressSpan = document.getElementById('walletAddress');
const bucketNameInput = document.getElementById('bucketName');
const createBucketBtn = document.getElementById('createBucket');
const bucketsList = document.getElementById('bucketsList');
const bucketSelect = document.getElementById('bucketSelect');
const fileInput = document.getElementById('fileInput');
const uploadFileBtn = document.getElementById('uploadFile');
const filesList = document.getElementById('filesList');

// State
let sdk = null;
let currentAddress = null;

// Initialize WASM and SDK
async function initialize() {
    try {
        await init();
        sdk = await AkaveWebSDK.new();
        console.log('SDK initialized successfully');
        connectWalletBtn.disabled = false;
    } catch (error) {
        console.error('Failed to initialize:', error);
        showError('Failed to initialize SDK. Please check the console for details.');
    }
}

// Connect Wallet
async function connectWallet() {
    try {
        if (!window.ethereum) {
            showError('No Ethereum wallet found. Please install MetaMask or another compatible wallet.');
            return;
        }

        // Request account access
        const accounts = await window.ethereum.request({ method: 'eth_requestAccounts' });
        currentAddress = accounts[0];
        
        // Update UI
        walletAddressSpan.textContent = `${currentAddress.slice(0, 6)}...${currentAddress.slice(-4)}`;
        connectWalletBtn.textContent = 'Connected';
        connectWalletBtn.disabled = true;

        // Load buckets
        await loadBuckets();
    } catch (error) {
        console.error('Failed to connect wallet:', error);
        showError('Failed to connect wallet. Please make sure your wallet is unlocked.');
    }
}

// Create Bucket
async function createBucket() {
    try {
        const bucketName = bucketNameInput.value.trim();
        if (!bucketName) {
            showError('Please enter a bucket name');
            return;
        }

        await sdk.createBucket(bucketName);
        showSuccess(`Bucket "${bucketName}" created successfully!`);
        bucketNameInput.value = '';
        await loadBuckets();
    } catch (error) {
        console.error('Failed to create bucket:', error);
        showError('Failed to create bucket. Please check the console for details.');
    }
}

// Load Buckets
async function loadBuckets() {
    try {
        const response = await sdk.listBuckets(currentAddress);
        bucketsList.innerHTML = '';
        bucketSelect.innerHTML = '<option value="">Select a bucket</option>';

        response.buckets.forEach(bucket => {
            // Add to buckets list
            const bucketElement = document.createElement('div');
            bucketElement.className = 'list-item';
            bucketElement.innerHTML = `
                <span>${bucket.name}</span>
                <button class="btn" onclick="deleteBucket('${bucket.name}')">Delete</button>
            `;
            bucketsList.appendChild(bucketElement);

            // Add to select dropdown
            const option = document.createElement('option');
            option.value = bucket.name;
            option.textContent = bucket.name;
            bucketSelect.appendChild(option);
        });
    } catch (error) {
        console.error('Failed to load buckets:', error);
        showError('Failed to load buckets. Please check the console for details.');
    }
}

// Delete Bucket
async function deleteBucket(bucketName) {
    try {
        await sdk.deleteBucket(currentAddress, bucketName);
        showSuccess(`Bucket "${bucketName}" deleted successfully!`);
        await loadBuckets();
    } catch (error) {
        console.error('Failed to delete bucket:', error);
        showError('Failed to delete bucket. Please check the console for details.');
    }
}

// Load Files
async function loadFiles(bucketName) {
    try {
        const response = await sdk.listFiles(currentAddress, bucketName);
        filesList.innerHTML = '';

        response.files.forEach(file => {
            const fileElement = document.createElement('div');
            fileElement.className = 'list-item';
            fileElement.innerHTML = `
                <span>${file.name}</span>
                <button class="btn" onclick="deleteFile('${bucketName}', '${file.name}')">Delete</button>
            `;
            filesList.appendChild(fileElement);
        });
    } catch (error) {
        console.error('Failed to load files:', error);
        showError('Failed to load files. Please check the console for details.');
    }
}

// Delete File
async function deleteFile(bucketName, fileName) {
    try {
        await sdk.deleteFile(currentAddress, bucketName, fileName);
        showSuccess(`File "${fileName}" deleted successfully!`);
        await loadFiles(bucketName);
    } catch (error) {
        console.error('Failed to delete file:', error);
        showError('Failed to delete file. Please check the console for details.');
    }
}

// Upload File
async function uploadFile() {
    try {
        const bucketName = bucketSelect.value;
        const file = fileInput.files[0];
        
        if (!bucketName || !file) {
            showError('Please select both a bucket and a file');
            return;
        }

        await sdk.uploadFile(bucketName, file);
        showSuccess(`File "${file.name}" uploaded successfully!`);
        await loadFiles(bucketName);
    } catch (error) {
        console.error('Failed to upload file:', error);
        showError('Failed to upload file. Please check the console for details.');
    }
}

// Event Listeners
connectWalletBtn.addEventListener('click', connectWallet);
createBucketBtn.addEventListener('click', createBucket);
bucketSelect.addEventListener('change', (e) => {
    if (e.target.value) {
        loadFiles(e.target.value);
        uploadFileBtn.disabled = false;
    } else {
        uploadFileBtn.disabled = true;
        filesList.innerHTML = '';
    }
});
uploadFileBtn.addEventListener('click', uploadFile);

// Utility Functions
function showError(message) {
    const errorDiv = document.createElement('div');
    errorDiv.className = 'error';
    errorDiv.textContent = message;
    document.body.appendChild(errorDiv);
    setTimeout(() => errorDiv.remove(), 5000);
}

function showSuccess(message) {
    const successDiv = document.createElement('div');
    successDiv.className = 'success';
    successDiv.textContent = message;
    document.body.appendChild(successDiv);
    setTimeout(() => successDiv.remove(), 5000);
}

// Initialize the application
initialize(); 