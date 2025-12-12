{-# LANGUAGE OverloadedStrings #-}

module Duplicates
    ( detectDuplicates
    , DuplicateGroup(..)
    ) where

import Scanner (FileInfo(..))
import Options (Options(..))
import qualified Data.Map.Strict as Map
import Data.List (sortBy, groupBy)
import Data.Function (on)
import Crypto.Hash (hash, MD5, Digest)
import qualified Data.ByteString.Lazy as B
import Control.Monad (forM)
import Data.Maybe (mapMaybe)

data DuplicateGroup = DuplicateGroup
    { dgHash :: String
    , dgFiles :: [FileInfo]
    , dgKeep :: FileInfo
    , dgDelete :: [FileInfo]
    } deriving (Show)

detectDuplicates :: Options -> [FileInfo] -> IO [DuplicateGroup]
detectDuplicates opts files = do
    -- Filter relevant files
    let candidates = filter isCandidate files

    -- Group by size first
    let bySize = groupBy ((==) `on` fiSize) $ sortBy (compare `on` fiSize) candidates

    groups <- forM bySize $ \sizeGroup -> do
        if length sizeGroup < 2
            then return []
            else do
                -- Calculate hashes or use fuzzy matching
                hashedGroups <- if optSkipCloudHash opts
                    then groupByFuzzy opts sizeGroup
                    else groupByHash sizeGroup

                return hashedGroups

    return (concat groups)

isCandidate :: FileInfo -> Bool
isCandidate info =
    not (fiIsFailedDownload info) &&
    not (fiIsTooSmall info) &&
    fiExtension info `elem` [".pdf", ".epub", ".txt"]

groupByHash :: [FileInfo] -> IO [DuplicateGroup]
groupByHash files = do
    hashedFiles <- forM files $ \f -> do
        h <- computeHash (fiOriginalPath f)
        return (h, f)

    let byHash = groupBy ((==) `on` fst) $ sortBy (compare `on` fst) hashedFiles

    return $ mapMaybe createGroup byHash

groupByFuzzy :: Options -> [FileInfo] -> IO [DuplicateGroup]
groupByFuzzy _ files = do
    -- Placeholder for fuzzy matching.
    -- For now, just group by exact name to be safe if fuzzy logic is complex to implement without deps.
    -- Or we can assume that if sizes are equal and names are similar, they are duplicates.
    -- Implementation: Group by normalized name (if available) or original name.
    let byName = groupBy ((==) `on` (fiOriginalName . snd)) $
                 sortBy (compare `on` (fiOriginalName . snd))
                 (map (\f -> ("", f)) files) -- dummy hash
    return $ mapMaybe createGroup byName

createGroup :: [(String, FileInfo)] -> Maybe DuplicateGroup
createGroup [] = Nothing
createGroup [_] = Nothing
createGroup items =
    let hashVal = fst (head items)
        files = map snd items
        -- Sort files to decide which to keep
        sortedFiles = sortBy prioritySort files
        keep = head sortedFiles
        delete = tail sortedFiles
    in Just DuplicateGroup
        { dgHash = hashVal
        , dgFiles = files
        , dgKeep = keep
        , dgDelete = delete
        }

prioritySort :: FileInfo -> FileInfo -> Ordering
prioritySort a b =
    -- Priority 1: Has new name (normalized)
    case (fiNewName a, fiNewName b) of
        (Just _, Nothing) -> LT
        (Nothing, Just _) -> GT
        _ ->
            -- Priority 2: Shorter path depth
            case compare (length (fiOriginalPath a)) (length (fiOriginalPath b)) of
                EQ ->
                    -- Priority 3: Newer modification time
                    compare (fiModifiedTime b) (fiModifiedTime a)
                other -> other

computeHash :: FilePath -> IO String
computeHash path = do
    content <- B.readFile path
    let digest = hash (B.toStrict content) :: Digest MD5
    return (show digest)
