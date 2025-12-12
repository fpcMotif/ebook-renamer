{-# LANGUAGE OverloadedStrings #-}

module Main (main) where

import Options
import Scanner
import Normalizer
import Duplicates
import Actions
import Control.Monad (forM, when)
import Data.Aeson (encode, object, (.=), ToJSON(..))
import qualified Data.ByteString.Lazy.Char8 as BL
import System.FilePath (makeRelative, (</>))
import Data.List (sort)

main :: IO ()
main = do
    -- 1. Parse CLI arguments
    opts <- parseOptions

    let quiet = optJson opts

    -- 2. Scan Directory
    when (not quiet) $ putStrLn $ "Scanning directory: " ++ optPath opts
    let scanOpts = ScanOptions
            { soMaxDepth = optMaxDepth opts
            , soRecursive = not (optNoRecursive opts)
            }

    files <- scanDirectory scanOpts (optPath opts)
    when (not quiet) $ putStrLn $ "Found " ++ show (length files) ++ " files"
    
    -- 3. Normalize Files
    let normalizedFiles = map normalizeFile files
    
    -- 4. Detect Duplicates
    duplicates <- detectDuplicates opts normalizedFiles
    when (not quiet) $ putStrLn $ "Found " ++ show (length duplicates) ++ " duplicate groups"
    
    -- 5. Execute Actions (Rename, Delete)
    applyChanges opts normalizedFiles duplicates
    
    -- 6. Generate Todo List
    generateTodo opts normalizedFiles

    -- 7. JSON Output
    if quiet
        then do
            let jsonOutput = generateJsonOutput opts normalizedFiles duplicates
            BL.putStrLn (encode jsonOutput)
        else putStrLn "Done!"

generateJsonOutput :: Options -> [FileInfo] -> [DuplicateGroup] -> JsonReport
generateJsonOutput opts files duplicates =
    let renames = [ RenameItem (makeRel old) (makeRel (dir </> new)) "normalized"
                  | f <- files
                  , let old = fiOriginalPath f
                  , let dir = takeDirectory old
                  , Just new <- [fiNewName f]
                  , old /= (dir </> new)
                  ]

        dupeDeletes = [ DuplicateDeleteItem (makeRel (fiOriginalPath (dgKeep g)))
                            [makeRel (fiOriginalPath d) | d <- dgDelete g]
                      | g <- duplicates
                      ]

        smallDeletes = if optDeleteSmall opts
                       then [ SmallDeleteItem (makeRel (fiOriginalPath f)) "deleted"
                            | f <- files, fiIsTooSmall f ]
                       else []

        todoItems = [ TodoItem "failed_download" (fiOriginalName f) ("重新下载: " ++ fiOriginalName f ++ " (未完成下载)")
                    | f <- files, fiIsFailedDownload f ] ++
                    [ TodoItem "too_small" (fiOriginalName f) ("检查并重新下载: " ++ fiOriginalName f ++ " (文件过小，仅 " ++ show (fiSize f) ++ " 字节)")
                    | f <- files, fiIsTooSmall f, not (optDeleteSmall opts) ]

    in JsonReport renames dupeDeletes smallDeletes todoItems
  where
    root = optPath opts
    makeRel p = makeRelative root p

data JsonReport = JsonReport
    { jrRenames :: [RenameItem]
    , jrDuplicateDeletes :: [DuplicateDeleteItem]
    , jrSmallDeletes :: [SmallDeleteItem]
    , jrTodoItems :: [TodoItem]
    }

instance ToJSON JsonReport where
    toJSON r = object
        [ "renames" .= jrRenames r
        , "duplicate_deletes" .= jrDuplicateDeletes r
        , "small_or_corrupted_deletes" .= jrSmallDeletes r
        , "todo_items" .= jrTodoItems r
        ]

data RenameItem = RenameItem String String String
instance ToJSON RenameItem where
    toJSON (RenameItem f t r) = object ["from" .= f, "to" .= t, "reason" .= r]

data DuplicateDeleteItem = DuplicateDeleteItem String [String]
instance ToJSON DuplicateDeleteItem where
    toJSON (DuplicateDeleteItem k d) = object ["keep" .= k, "delete" .= d]

data SmallDeleteItem = SmallDeleteItem String String
instance ToJSON SmallDeleteItem where
    toJSON (SmallDeleteItem p i) = object ["path" .= p, "issue" .= i]

data TodoItem = TodoItem String String String
instance ToJSON TodoItem where
    toJSON (TodoItem c f m) = object ["category" .= c, "file" .= f, "message" .= m]
