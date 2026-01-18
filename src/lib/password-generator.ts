/**
 * Secure password generator using Web Crypto API.
 */

export interface PasswordOptions {
  length: number;
  uppercase: boolean;
  lowercase: boolean;
  numbers: boolean;
  symbols: boolean;
}

const CHAR_SETS = {
  uppercase: "ABCDEFGHIJKLMNOPQRSTUVWXYZ",
  lowercase: "abcdefghijklmnopqrstuvwxyz",
  numbers: "0123456789",
  symbols: "!@#$%^&*()_+-=[]{}|;:,.<>?"
};

export const DEFAULT_OPTIONS: PasswordOptions = {
  length: 24,
  uppercase: true,
  lowercase: true,
  numbers: true,
  symbols: true
};

type CharsetKey = keyof typeof CHAR_SETS;

/**
 * Generates a cryptographically secure random password.
 * @param options Password generation options
 * @returns Generated password string
 */
export function generatePassword(options: PasswordOptions = DEFAULT_OPTIONS): string {
  const enabledSets = getEnabledCharsets(options);
  const charset = enabledSets.map((key) => CHAR_SETS[key]).join("");
  const requiredChars = enabledSets.map((key) => getRandomChar(CHAR_SETS[key]));

  if (charset.length === 0) {
    throw new Error("At least one character type must be selected");
  }

  if (options.length < requiredChars.length) {
    throw new Error(`Password length must be at least ${requiredChars.length} to include all character types`);
  }

  const remainingLength = options.length - requiredChars.length;
  const randomChars: string[] = [];

  const randomValues = getRandomValues(remainingLength);
  for (let i = 0; i < remainingLength; i++) {
    randomChars.push(charset[randomValues[i] % charset.length]);
  }

  const allChars = [...requiredChars, ...randomChars];
  return shuffleArray(allChars).join("");
}

function getEnabledCharsets(options: PasswordOptions): CharsetKey[] {
  const enabled: CharsetKey[] = [];
  if (options.uppercase) enabled.push("uppercase");
  if (options.lowercase) enabled.push("lowercase");
  if (options.numbers) enabled.push("numbers");
  if (options.symbols) enabled.push("symbols");
  return enabled;
}

/**
 * Gets a random character from the given charset.
 */
function getRandomChar(charset: string): string {
  const randomValue = getRandomValues(1)[0];
  return charset[randomValue % charset.length];
}

function getRandomValues(size: number): Uint32Array {
  const randomValues = new Uint32Array(size);
  crypto.getRandomValues(randomValues);
  return randomValues;
}

/**
 * Shuffles an array using Fisher-Yates algorithm with crypto-random values.
 */
function shuffleArray<T>(array: T[]): T[] {
  const shuffled = [...array];
  const randomValues = getRandomValues(shuffled.length);

  for (let i = shuffled.length - 1; i > 0; i--) {
    const j = randomValues[i] % (i + 1);
    [shuffled[i], shuffled[j]] = [shuffled[j], shuffled[i]];
  }
  return shuffled;
}

/**
 * Calculates password strength based on length and character variety.
 */
export function calculateStrength(password: string): "weak" | "fair" | "good" | "strong" {
  let score = 0;

  if (password.length >= 8) score++;
  if (password.length >= 12) score++;
  if (password.length >= 20) score++;
  if (/[a-z]/.test(password)) score++;
  if (/[A-Z]/.test(password)) score++;
  if (/[0-9]/.test(password)) score++;
  if (/[^a-zA-Z0-9]/.test(password)) score++;

  if (score <= 2) return "weak";
  if (score <= 4) return "fair";
  if (score <= 6) return "good";
  return "strong";
}
