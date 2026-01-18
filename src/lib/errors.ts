/**
 * Error message mapping for user-friendly error messages.
 * Maps technical backend errors to human-readable messages.
 */

interface ErrorMapping {
  pattern: RegExp;
  message: string;
}

const ERROR_MAP: ErrorMapping[] = [
  // Password/authentication errors
  { pattern: /load:.*Crypto/i, message: "Incorrect password. Please try again." },
  { pattern: /current master password is incorrect/i, message: "Current password is incorrect. Please try again." },

  // Vault state errors
  { pattern: /vault is locked/i, message: "Your vault is locked. Please unlock it first." },
  { pattern: /vault already exists/i, message: "A vault already exists. Please unlock it or delete the existing vault." },
  { pattern: /vault does not exist/i, message: "No vault found. Please create a new vault first." },

  // File operation errors
  { pattern: /load:.*Io/i, message: "Unable to open the file. Please check the file path and try again." },
  { pattern: /save:.*Io/i, message: "Unable to save the vault. Please check disk space and permissions." },
  { pattern: /export:.*Io/i, message: "Unable to export backup. Please check the file path and permissions." },
  { pattern: /import:.*Io/i, message: "Unable to import backup. Please check the file path and try again." },
  { pattern: /create_dir_all failed/i, message: "Unable to create directory. Please check permissions." },

  // Format errors
  { pattern: /Format.*too small/i, message: "Invalid vault file. The file may be corrupted or not a valid backup." },
  { pattern: /Format/i, message: "Invalid file format. Please ensure you selected a valid vault backup." },

  // KDF errors
  { pattern: /kdf:/i, message: "Error processing password. Please try again." },

  // Rate limiting
  { pattern: /Too many failed attempts/i, message: "Too many failed attempts. Please wait before trying again." },

  // Entry errors
  { pattern: /entry not found/i, message: "Entry not found. It may have been deleted." },

  // Clipboard errors
  { pattern: /clipboard/i, message: "Unable to access clipboard. Please try copying manually." },

  // Mutex/concurrency errors
  { pattern: /mutex poisoned/i, message: "An internal error occurred. Please restart the application." },

  // Path errors
  { pattern: /path is required/i, message: "Please enter a file path." },
];

/**
 * Converts a raw technical error message to a user-friendly message.
 * @param rawError The raw error string from the backend
 * @returns A user-friendly error message
 */
export function friendlyError(rawError: string): string {
  for (const { pattern, message } of ERROR_MAP) {
    if (pattern.test(rawError)) {
      return message;
    }
  }

  // Fallback: log the original error for debugging and return a generic message
  console.error("Unmapped error:", rawError);
  return "An unexpected error occurred. Please try again.";
}
