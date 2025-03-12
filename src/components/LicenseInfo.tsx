import React, { useState } from 'react';
import './LicenseInfo.css';

interface LicenseInfoProps {
  licenseType: 'free' | 'pro';
  onActivate: (licenseKey: string, email: string) => void;
}

const LicenseInfo: React.FC<LicenseInfoProps> = ({ licenseType, onActivate }) => {
  const [showActivation, setShowActivation] = useState(false);
  const [licenseKey, setLicenseKey] = useState('');
  const [email, setEmail] = useState('');
  const [isSubmitting, setIsSubmitting] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    
    if (!licenseKey || !email) {
      alert('Please enter both license key and email');
      return;
    }
    
    setIsSubmitting(true);
    
    try {
      await onActivate(licenseKey, email);
      // If successful, we'll get a new license status from the parent component
      // which will update the UI accordingly
      setShowActivation(false);
      setLicenseKey('');
      setEmail('');
    } catch (error) {
      console.error('License activation error:', error);
    } finally {
      setIsSubmitting(false);
    }
  };

  // Show different content based on license type
  return (
    <div className="license-info-container">
      <div className="license-info-header">
        <h2>License Information</h2>
        {licenseType === 'free' && (
          <button 
            className="upgrade-button"
            onClick={() => setShowActivation(!showActivation)}
          >
            {showActivation ? 'Cancel' : 'Activate License'}
          </button>
        )}
      </div>
      
      <div className="license-status">
        {licenseType === 'pro' ? (
          <div className="pro-license-info">
            <h3>Pro License Active</h3>
            <p>Thank you for using Rustloader Pro!</p>
            <ul className="license-features">
              <li>✅ High-quality downloads (1080p, 4K)</li>
              <li>✅ Unlimited daily downloads</li>
              <li>✅ Additional audio formats</li>
              <li>✅ Priority support</li>
            </ul>
          </div>
        ) : (
          <div className="free-license-info">
            <h3>Free License</h3>
            <p>You're using the free version of Rustloader.</p>
            <ul className="license-features">
              <li>✅ Downloads up to 720p</li>
              <li>✅ Basic MP3 audio (128kbps)</li>
              <li>✅ Limited to 5 downloads per day</li>
              <li>❌ High-quality video (1080p, 4K)</li>
            </ul>
            <p className="pro-promo">
              Upgrade to Pro for unlimited downloads and higher quality!
            </p>
          </div>
        )}
      </div>
      
      {showActivation && licenseType === 'free' && (
        <div className="license-activation">
          <h3>Activate Pro License</h3>
          <form onSubmit={handleSubmit}>
            <div className="form-group">
              <label htmlFor="license-key">License Key</label>
              <input
                type="text"
                id="license-key"
                value={licenseKey}
                onChange={(e) => setLicenseKey(e.target.value)}
                placeholder="PRO-XXXX-XXXX-XXXX"
                disabled={isSubmitting}
                required
              />
            </div>
            
            <div className="form-group">
              <label htmlFor="email">Email</label>
              <input
                type="email"
                id="email"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                placeholder="your@email.com"
                disabled={isSubmitting}
                required
              />
            </div>
            
            <div className="form-group">
              <button 
                type="submit" 
                className="activate-button"
                disabled={isSubmitting || !licenseKey || !email}
              >
                {isSubmitting ? 'Activating...' : 'Activate License'}
              </button>
            </div>
          </form>
          
          <p className="purchase-note">
            Don't have a license? <a href="https://rustloader.com/pro" target="_blank" rel="noreferrer">Purchase Pro</a>
          </p>
        </div>
      )}
    </div>
  );
};

export default LicenseInfo;