{-# LANGUAGE OverloadedStrings #-}

module Scanner
    ( FileInfo(..)
    , ScanOptions(..)
    , scanDirectory
    ) where

import System.Directory
import System.FilePath
import Data.Time.Clock (UTCTime)
import Control.Monad (forM, foldM)
import Data.List (isPrefixOf)
import Control.Exception (try, IOException)

data FileInfo = FileInfo
    { fiOriginalPath     :: FilePath
    , fiOriginalName     :: String
    , fiExtension        :: String
    , fiSize             :: Integer
    , fiModifiedTime     :: UTCTime
    , fiIsFailedDownload :: Bool
    , fiIsTooSmall       :: Bool
    , fiNewName          :: Maybe String
    , fiNewPath          :: Maybe FilePath
    } deriving (Show, Eq)

data ScanOptions = ScanOptions
    { soMaxDepth :: Int
    , soRecursive :: Bool
    } deriving (Show, Eq)

ignoredDirs :: [String]
ignoredDirs = ["Xcode", "node_modules", ".git", "__pycache__"]

isIgnoredDir :: FilePath -> Bool
isIgnoredDir path =
    let dirName = takeFileName path
    in "." `isPrefixOf` dirName || dirName `elem` ignoredDirs

scanDirectory :: ScanOptions -> FilePath -> IO [FileInfo]
scanDirectory opts root = do
    exists <- doesDirectoryExist root
    if not exists
        then return []
        else scanRecursive opts root 0

scanRecursive :: ScanOptions -> FilePath -> Int -> IO [FileInfo]
scanRecursive opts dir depth
    | depth > soMaxDepth opts = return []
    | otherwise = do
        contents <- try (listDirectory dir) :: IO (Either IOException [FilePath])
        case contents of
            Left _ -> return []
            Right items -> do
                files <- foldM (processItem opts dir depth) [] items
                return files

processItem :: ScanOptions -> FilePath -> Int -> [FileInfo] -> FilePath -> IO [FileInfo]
processItem opts dir depth acc item = do
    let path = dir </> item

    -- Check if it's a directory
    isDir <- doesDirectoryExist path
    if isDir
        then do
            if isIgnoredDir path
                then return acc
                else if soRecursive opts
                    then do
                        subFiles <- scanRecursive opts path (depth + 1)
                        return (acc ++ subFiles)
                    else return acc
        else do
            -- It's a file
            if "." `isPrefixOf` item
                then return acc -- Skip hidden files
                else do
                    info <- getFileInfo path item
                    return (info : acc)

getFileInfo :: FilePath -> String -> IO FileInfo
getFileInfo path name = do
    size <- getFileSize path
    modTime <- getModificationTime path

    let ext = takeExtension name
    let isFailed = ext `elem` [".download", ".crdownload"]
    let isPdfOrEpub = ext `elem` [".pdf", ".epub"]
    let isSmall = isPdfOrEpub && size < 1024 && not isFailed

    return FileInfo
        { fiOriginalPath = path
        , fiOriginalName = name
        , fiExtension = ext
        , fiSize = size
        , fiModifiedTime = modTime
        , fiIsFailedDownload = isFailed
        , fiIsTooSmall = isSmall
        , fiNewName = Nothing
        , fiNewPath = Nothing
        }
