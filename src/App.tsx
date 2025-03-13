import React, { useState, useEffect } from 'react';

// Define component prop types
interface DownloadFormProps {
  isPro: boolean;
  onDownloadStart: () => void;
}

// Simplified DownloadForm component
const DownloadForm: React.FC<DownloadFormProps> = ({ isPro, onDownloadStart }) => {
  const [url, setUrl] = useState('');
  const [isLoading, setIsLoading] = useState(false);

  const handleSubmit = (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    if (!url) return;
    
    setIsLoading(true);
    // Simulate download start
    setTimeout(() => {
      onDownloadStart();
      setIsLoading(false);
    }, 500);
  };

  return (
    <div className="bg-white dark:bg-gray-800 rounded-lg shadow-md p-6">
      <form onSubmit={handleSubmit}>
        <div className="mb-4">
          <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
            Video URL
          </label>
          <input
            type="text"
            value={url}
            onChange={(e) => setUrl(e.target.value)}
            placeholder="https://www.youtube.com/watch?v=..."
            className="w-full p-2 border rounded-md text-sm dark:bg-gray-700 dark:border-gray-600 dark:text-white"
            required
          />
        </div>
        <div className="mb-4">
          <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
            Quality
          </label>
          <select
            className="w-full p-2 border rounded-md text-sm dark:bg-gray-700 dark:border-gray-600 dark:text-white"
          >
            <option value="480">480p</option>
            <option value="720">720p</option>
            {isPro && (
              <>
                <option value="1080">1080p</option>
                <option value="2160">4K</option>
              </>
            )}
          </select>
        </div>
        <button
          type="submit"
          disabled={isLoading || !url}
          className="w-full py-2 px-4 bg-blue-600 hover:bg-blue-700 text-white font-medium rounded-md shadow-sm disabled:bg-blue-300 transition-colors"
        >
          {isLoading ? 'Processing...' : 'Download'}
        </button>
      </form>
    </div>
  );
};

// Define license activation props
interface LicenseActivationProps {
  isProVersion: boolean;
  onActivationComplete: (success: boolean) => void;
}

// Simplified LicenseActivation component
const LicenseActivation: React.FC<LicenseActivationProps> = ({ isProVersion, onActivationComplete }) => {
  const [licenseKey, setLicenseKey] = useState('');
  const [email, setEmail] = useState('');
  const [isActivating, setIsActivating] = useState(false);

  const handleActivate = (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    if (!licenseKey || !email) return;
    
    setIsActivating(true);
    // Simulate activation
    setTimeout(() => {
      onActivationComplete(true);
      setIsActivating(false);
    }, 1500);
  };

  if (isProVersion) {
    return (
      <div className="bg-green-50 dark:bg-green-900 p-6 rounded-lg shadow-md">
        <h2 className="text-lg font-semibold text-green-800 dark:text-green-200 mb-2">
          Pro License Active
        </h2>
        <p className="text-green-700 dark:text-green-300">
          Thank you for using Rustloader Pro!
        </p>
      </div>
    );
  }

  return (
    <div className="bg-white dark:bg-gray-800 p-6 rounded-lg shadow-md">
      <h2 className="text-lg font-semibold text-gray-800 dark:text-white mb-4">
        Activate Pro License
      </h2>
      <form onSubmit={handleActivate} className="space-y-4">
        <div>
          <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
            License Key
          </label>
          <input
            type="text"
            value={licenseKey}
            onChange={(e) => setLicenseKey(e.target.value)}
            placeholder="PRO-XXXX-XXXX-XXXX"
            className="w-full p-2 border rounded-md text-sm dark:bg-gray-700 dark:border-gray-600 dark:text-white"
          />
        </div>
        <div>
          <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
            Email
          </label>
          <input
            type="email"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            placeholder="your@email.com"
            className="w-full p-2 border rounded-md text-sm dark:bg-gray-700 dark:border-gray-600 dark:text-white"
          />
        </div>
        <button
          type="submit"
          disabled={isActivating || !licenseKey || !email}
          className="w-full py-2 px-4 bg-blue-600 hover:bg-blue-700 text-white font-medium rounded-md shadow-sm disabled:bg-blue-300 transition-colors"
        >
          {isActivating ? 'Activating...' : 'Activate License'}
        </button>
      </form>
    </div>
  );
};

// Define progress bar props
interface ProgressBarProps {
  progress: number;
}

// Simplified ProgressBar component
const ProgressBar: React.FC<ProgressBarProps> = ({ progress }) => {
  const normalizedProgress = Math.min(100, Math.max(0, progress || 0));
  
  return (
    <div className="bg-white dark:bg-gray-800 rounded-lg shadow-md p-6">
      <div className="flex justify-between mb-2">
        <span className="text-sm font-medium text-gray-700 dark:text-gray-300">Downloading...</span>
        <span className="text-sm font-medium text-blue-600 dark:text-blue-400">{normalizedProgress.toFixed(1)}%</span>
      </div>
      <div className="h-2.5 bg-gray-200 dark:bg-gray-700 rounded-full">
        <div 
          className="h-2.5 bg-blue-600 rounded-full" 
          style={{ width: `${normalizedProgress}%` }}
        ></div>
      </div>
      <div className="mt-3 text-xs text-gray-500 dark:text-gray-400">
        {normalizedProgress >= 100 ? 'Processing file...' : 'Downloading...'}
      </div>
    </div>
  );
};

// Main App component
const App: React.FC = () => {
  const [licenseStatus, setLicenseStatus] = useState<'free' | 'pro'>('free'); // 'free' or 'pro'
  const [isDownloading, setIsDownloading] = useState(false);
  const [downloadProgress, setDownloadProgress] = useState(0);
  const [activeTab, setActiveTab] = useState<'download' | 'license'>('download'); // 'download' or 'license'
  
  // Simulate checking license status on component mount
  useEffect(() => {
    // In a real app, this would check with a backend API
    const checkLicense = async (): Promise<void> => {
      // Simulate API call delay
      await new Promise(resolve => setTimeout(resolve, 1000));
      
      // For demonstration, we'll default to 'free'
      // In a real app, this would be set based on license verification
      setLicenseStatus('free');
    };
    
    checkLicense();
  }, []);
  
  // Simulate download progress updates
  useEffect(() => {
    if (!isDownloading) return;
    
    // Simulate progress updates
    const interval = setInterval(() => {
      setDownloadProgress(prev => {
        const newProgress = prev + (Math.random() * 2);
        
        if (newProgress >= 100) {
          clearInterval(interval);
          
          // Simulate processing after download completes
          setTimeout(() => {
            setIsDownloading(false);
            setDownloadProgress(0);
          }, 3000);
          
          return 100;
        }
        return newProgress;
      });
    }, 500);
    
    return () => clearInterval(interval);
  }, [isDownloading]);
  
  // Handle download start
  const handleDownloadStart = (): void => {
    setIsDownloading(true);
    setDownloadProgress(0);
  };
  
  // Handle license activation
  const handleLicenseActivation = (success: boolean): void => {
    if (success) {
      setLicenseStatus('pro');
    }
  };
  
  return (
    <div className="min-h-screen bg-gray-100 dark:bg-gray-900 py-8 px-4">
      <div className="max-w-4xl mx-auto">
        {/* Header */}
        <header className="text-center mb-8">
          <h1 className="text-3xl font-bold text-gray-800 dark:text-white mb-2">
            Rustloader
          </h1>
          <p className="text-gray-600 dark:text-gray-400">
            Advanced Video Downloader
          </p>
          
          {/* License Badge */}
          <div className="mt-3">
            <span className={`inline-block px-3 py-1 text-sm font-medium text-white rounded-full ${
              licenseStatus === 'pro' ? 'bg-yellow-500' : 'bg-blue-500'
            }`}>
              {licenseStatus === 'pro' ? 'PRO VERSION' : 'FREE VERSION'}
            </span>
          </div>
        </header>
        
        {/* Tab Navigation */}
        <div className="flex border-b border-gray-200 dark:border-gray-700 mb-6">
          <button
            className={`py-2 px-4 font-medium text-sm ${
              activeTab === 'download'
                ? 'text-blue-600 dark:text-blue-400 border-b-2 border-blue-600 dark:border-blue-400'
                : 'text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300'
            }`}
            onClick={() => setActiveTab('download')}
          >
            Download
          </button>
          <button
            className={`py-2 px-4 font-medium text-sm ${
              activeTab === 'license'
                ? 'text-blue-600 dark:text-blue-400 border-b-2 border-blue-600 dark:border-blue-400'
                : 'text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300'
            }`}
            onClick={() => setActiveTab('license')}
          >
            License
          </button>
        </div>
        
        {/* Main Content */}
        <div className="space-y-6">
          {/* Progress Bar (shown during download) */}
          {isDownloading && (
            <ProgressBar progress={downloadProgress} />
          )}
          
          {/* Active Tab Content */}
          {activeTab === 'download' ? (
            <DownloadForm 
              isPro={licenseStatus === 'pro'} 
              onDownloadStart={handleDownloadStart}
            />
          ) : (
            <LicenseActivation 
              isProVersion={licenseStatus === 'pro'} 
              onActivationComplete={handleLicenseActivation}
            />
          )}
          
          {/* Info Card */}
          <div className="bg-blue-50 dark:bg-blue-900 p-4 rounded-lg shadow-sm">
            <h3 className="text-sm font-medium text-blue-800 dark:text-blue-200 mb-1">
              Rustloader v1.0.0
            </h3>
            <p className="text-xs text-blue-600 dark:text-blue-300">
              Advanced Video Downloader built with Rust and React
            </p>
          </div>
        </div>
      </div>
    </div>
  );
};

export default App;