import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { listen } from '@tauri-apps/api/event';
import './App.css';
import DownloadForm from './components/DownloadForm';
import LicenseInfo from './components/LicenseInfo';
import ProgressBar from './components/ProgressBar';

function App() {
  const [licenseType, setLicenseType] = useState<'free' | 'pro'>('free');
  const [isDownloading, setIsDownloading] = useState(false);
  const [downloadProgress, setDownloadProgress] = useState(0);
  const [message, setMessage] = useState('');

  // Check license status when app loads
  useEffect(() => {
    const checkLicense = async () => {
      try {
        const status = await invoke<string>('check_license');
        setLicenseType(status as 'free' | 'pro');
      } catch (error) {
        console.error('Failed to check license:', error);
      }
    };
    
    checkLicense();
  }, []);

  // Set up download progress listener
  useEffect(() => {
    const setupListener = async () => {
      const unlisten = await listen('download-progress', (event) => {
        setDownloadProgress(event.payload as number);
      });

      return () => {
        unlisten();
      };
    };

    const cleanup = setupListener();
    return () => {
      cleanup.then(unlisten => unlisten);
    };
  }, []);

  // Handle download submission
  const handleDownload = async (downloadOptions: {
    url: string;
    quality: string;
    format: string;
    startTime?: string;
    endTime?: string;
    usePlaylist: boolean;
    downloadSubtitles: boolean;
    outputDir?: string;
  }) => {
    setIsDownloading(true);
    setMessage('');
    setDownloadProgress(0);
    
    try {
      const result = await invoke<string>('download_video', {
        url: downloadOptions.url,
        quality: downloadOptions.quality || null,
        format: downloadOptions.format,
        startTime: downloadOptions.startTime || null,
        endTime: downloadOptions.endTime || null,
        usePlaylist: downloadOptions.usePlaylist,
        downloadSubtitles: downloadOptions.downloadSubtitles,
        outputDir: downloadOptions.outputDir || null,
      });
      
      setMessage(result);
    } catch (error) {
      setMessage(`Error: ${error}`);
    } finally {
      setIsDownloading(false);
      setDownloadProgress(0);
    }
  };

  // Handle license activation
  const activateLicense = async (licenseKey: string, email: string) => {
    try {
      const result = await invoke<string>('activate_license_key', {
        licenseKey,
        email,
      });
      
      setMessage(result);
      
      // Re-check license status
      const status = await invoke<string>('check_license');
      setLicenseType(status as 'free' | 'pro');
    } catch (error) {
      setMessage(`Activation Error: ${error}`);
    }
  };

  return (
    <div className="app-container">
      <header>
        <h1>Rustloader</h1>
        <div className="license-badge">
          {licenseType === 'pro' ? 
            <span className="pro-badge">PRO</span> : 
            <span className="free-badge">FREE</span>
          }
        </div>
      </header>
      
      <main>
        <DownloadForm 
          onSubmit={handleDownload} 
          isPro={licenseType === 'pro'} 
          isDisabled={isDownloading}
        />
        
        {isDownloading && (
          <ProgressBar 
            progress={downloadProgress} 
            label="Downloading..." 
          />
        )}
        
        {message && (
          <div className={message.includes('Error') ? 'error-message' : 'success-message'}>
            {message}
          </div>
        )}
        
        <LicenseInfo 
          licenseType={licenseType} 
          onActivate={activateLicense} 
        />
      </main>
      
      <footer>
        <p>Rustloader v1.0.0 - Advanced Video Downloader</p>
      </footer>
    </div>
  );
}

export default App;