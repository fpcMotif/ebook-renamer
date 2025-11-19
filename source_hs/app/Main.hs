module Main (main) where

import System.IO (hPutStrLn, stderr)
import System.Environment (getArgs)
import Data.Time.Clock (getCurrentTime)
import Data.Time.Format (formatTime, defaultTimeLocale)

-- Logging helper
logInfo :: String -> IO ()
logInfo msg = do
    now <- getCurrentTime
    let timestamp = formatTime defaultTimeLocale "%Y-%m-%d %H:%M:%S" now
    hPutStrLn stderr $ "[" ++ timestamp ++ "] INFO: " ++ msg

main :: IO ()
main = do
    logInfo "Starting ebook renamer"
    
    args <- getArgs
    let path = if null args then "." else head args
    
    logInfo $ "Processing path: " ++ path
    
    -- For now, this is a minimal implementation showing the structure
    -- Full implementation would include:
    -- - CLI argument parsing using optparse-applicative
    -- - File scanning with recursion
    -- - Filename normalization
    -- - Duplicate detection
    -- - Todo list generation
    
    putStrLn "Haskell implementation - work in progress"
    putStrLn "This is a placeholder showing the logging structure"
    putStrLn "Full implementation requires:"
    putStrLn "  - CLI parsing module"
    putStrLn "  - Scanner module"
    putStrLn "  - Normalizer module"
    putStrLn "  - Duplicates module"
    putStrLn "  - Todo module"
