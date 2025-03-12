import React, { useState } from 'react';
import { open } from '@tauri-apps/api/dialog';
import './DownloadForm.css';

interface DownloadFormProps {
  onSubmit: (options: {
    url: string;
    quality: string;
    format: string;
    startTime?: string;
    endTime?: string;
    usePlaylist: boolean;
    downloadSubtitles: boolean;
    outputDir?: string;
  }) => void;
  isPro: boolean;
  isDisabled: boolean;
}

const DownloadForm: React.FC<DownloadFormProps> = ({ onSubmit, isPro, isDisabled }) => {
  const [url, setUrl] = useState('');
  const [quality, setQuality] = useState('720');
  const [format, setFormat] = useState('mp4');
  const [startTime, setStartTime] = useState('');
  const [endTime, setEndTime] = useState('');
  const [usePlaylist, setUsePlaylist] = useState(false);
  const [downloadSubtitles, setDownloadSubtitles] = useState(false);
  const [outputDir, setOutputDir] = useState<string | undefined>(undefined);
  const [showAdvanced, setShowAdvanced] = useState(false);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    
    if (!url) {
      alert('Please enter a URL');
      return;
    }
    
    onSubmit({
      url,
      quality,
      format,
      startTime: startTime || undefined,
      endTime: endTime || undefined,
      usePlaylist,
      downloadSubtitles,
      outputDir,
    });
  };

  const selectOutputDirectory = async () => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: 'Select Output Directory',
      });
      
      if (selected && !Array.isArray(selected)) {
        setOutputDir(selected);
      }
    } catch (error) {
      console.error('Failed to select directory:', error);
    }
  };

  const validateTimeFormat = (value: string) => {
    if (!value) return true;
    
    // Format HH:MM:SS
    const timeRegex = /^([0-9][0-9]):([0-5][0-9]):([0-5][0-9])$/;
    return timeRegex.test(value);
  };

  return (
    <div className="download-form-container">
      <form onSubmit={handleSubmit} className="download-form">
        <div className="form-group">
          <label htmlFor="url">Video URL</label>
          <input
            type="text"
            id="url"
            value={url}
            onChange={(e) => setUrl(e.target.value)}
            placeholder="https://www.youtube.com/watch?v=..."
            disabled={isDisabled}
            required
          />
        </div>
        
        <div className="form-row">
          <div className="form-group">
            <label htmlFor="quality">Quality</label>
            <select
              id="quality"
              value={quality}
              onChange={(e) => setQuality(e.target.value)}
              disabled={isDisabled}
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
            {!isPro && (
              <small className="pro-note">Pro version required for 1080p/4K</small>
            )}
          </div>
          
          <div className="form-group">
            <label htmlFor="format">Format</label>
            <select
              id="format"
              value={format}
              onChange={(e) => setFormat(e.target.value)}
              disabled={isDisabled}
            >
              <option value="mp4">MP4 Video</option>
              <option value="mp3">MP3 Audio</option>
              {isPro && (
                <>
                  <option value="webm">WebM</option>
                  <option value="flac">FLAC Audio</option>
                </>
              )}
            </select>
          </div>
        </div>
        
        <div className="form-group toggle-container">
          <button 
            type="button" 
            className="toggle-advanced"
            onClick={() => setShowAdvanced(!showAdvanced)}
            disabled={isDisabled}
          >
            {showAdvanced ? 'Hide Advanced Options' : 'Show Advanced Options'}
          </button>
        </div>
        
        {showAdvanced && (
          <div className="advanced-options">
            <div className="form-row">
              <div className="form-group">
                <label htmlFor="start-time">Start Time (HH:MM:SS)</label>
                <input
                  type="text"
                  id="start-time"
                  value={startTime}
                  onChange={(e) => setStartTime(e.target.value)}
                  placeholder="00:00:00"
                  pattern="^([0-9][0-9]):([0-5][0-9]):([0-5][0-9])$"
                  disabled={isDisabled}
                />
                {startTime && !validateTimeFormat(startTime) && (
                  <small className="error-hint">Use format HH:MM:SS</small>
                )}
              </div>
              
              <div className="form-group">
                <label htmlFor="end-time">End Time (HH:MM:SS)</label>
                <input
                  type="text"
                  id="end-time"
                  value={endTime}
                  onChange={(e) => setEndTime(e.target.value)}
                  placeholder="00:00:00"
                  pattern="^([0-9][0-9]):([0-5][0-9]):([0-5][0-9])$"
                  disabled={isDisabled}
                />
                {endTime && !validateTimeFormat(endTime) && (
                  <small className="error-hint">Use format HH:MM:SS</small>
                )}
              </div>
            </div>
            
            <div className="form-row">
              <div className="form-group checkbox-group">
                <label>
                  <input
                    type="checkbox"
                    checked={usePlaylist}
                    onChange={(e) => setUsePlaylist(e.target.checked)}
                    disabled={isDisabled}
                  />
                  Download entire playlist
                </label>
              </div>
              
              <div className="form-group checkbox-group">
                <label>
                  <input
                    type="checkbox"
                    checked={downloadSubtitles}
                    onChange={(e) => setDownloadSubtitles(e.target.checked)}
                    disabled={isDisabled}
                  />
                  Download subtitles
                </label>
              </div>
            </div>
            
            <div className="form-group">
              <label htmlFor="output-dir">Output Directory</label>
              <div className="directory-selector">
                <input
                  type="text"
                  id="output-dir"
                  value={outputDir || ''}
                  readOnly
                  placeholder="Default directory"
                  disabled={isDisabled}
                />
                <button
                  type="button"
                  onClick={selectOutputDirectory}
                  disabled={isDisabled}
                >
                  Browse
                </button>
              </div>
            </div>
          </div>
        )}
        
        <div className="form-group button-container">
          <button
            type="submit"
            className="download-button"
            disabled={isDisabled || !url || (startTime !== '' && !validateTimeFormat(startTime)) || (endTime !== '' && !validateTimeFormat(endTime))}
          >
            {isDisabled ? 'Downloading...' : 'Download'}
          </button>
        </div>
      </form>
    </div>
  );
};

export default DownloadForm;