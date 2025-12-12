{-# LANGUAGE OverloadedStrings #-}

module Actions
    ( applyChanges
    , generateTodo
    ) where

import Scanner (FileInfo(..))
import Duplicates (DuplicateGroup(..))
import Options (Options(..))
import System.Directory (renameFile, removeFile, createDirectoryIfMissing)
import System.FilePath (takeDirectory, (</>))
import Control.Monad (when, forM_)
import Data.Time.Clock (getCurrentTime)
import Data.Time.Format (formatTime, defaultTimeLocale)
import System.IO (writeFile)

applyChanges :: Options -> [FileInfo] -> [DuplicateGroup] -> IO ()
applyChanges opts files duplicates = do
    let quiet = optJson opts

    -- 1. Handle Renames
    let toRename = filter needsRename files
    forM_ toRename $ \f -> do
        case fiNewName f of
            Just newName -> do
                let oldPath = fiOriginalPath f
                    newPath = takeDirectory oldPath </> newName
                if oldPath /= newPath
                    then do
                        if optDryRun opts
                            then when (not quiet) $ putStrLn $ "Rename: " ++ oldPath ++ " -> " ++ newName
                            else do
                                renameFile oldPath newPath
                                when (not quiet) $ putStrLn $ "Renamed: " ++ newName
                    else return ()
            Nothing -> return ()

    -- 2. Handle Deletes
    when (not (optNoDelete opts)) $ do
        forM_ duplicates $ \g -> do
            forM_ (dgDelete g) $ \f -> do
                if optDryRun opts
                    then when (not quiet) $ putStrLn $ "Delete duplicate: " ++ fiOriginalPath f
                    else do
                        removeFile (fiOriginalPath f)
                        when (not quiet) $ putStrLn $ "Deleted: " ++ fiOriginalPath f

    -- 3. Handle Small/Corrupted Deletes
    when (optDeleteSmall opts) $ do
        let toDelete = filter (\f -> fiIsTooSmall f) files
        forM_ toDelete $ \f -> do
             if optDryRun opts
                then when (not quiet) $ putStrLn $ "Delete small: " ++ fiOriginalPath f
                else do
                    removeFile (fiOriginalPath f)
                    when (not quiet) $ putStrLn $ "Deleted small file: " ++ fiOriginalPath f

needsRename :: FileInfo -> Bool
needsRename f =
    case fiNewName f of
        Just _ -> True
        Nothing -> False

generateTodo :: Options -> [FileInfo] -> IO ()
generateTodo opts files = do
    now <- getCurrentTime
    let timestamp = formatTime defaultTimeLocale "%Y-%m-%d %H:%M:%S" now

        failed = filter fiIsFailedDownload files
        small = filter (\f -> fiIsTooSmall f && not (optDeleteSmall opts)) files

        content = unlines
            [ "# 需要检查的任务"
            , ""
            , "更新时间: " ++ timestamp
            , ""
            , "## 🔄 未完成下载文件（.download）"
            ]
            ++ unlines (map formatFailed failed)
            ++ unlines
            [ ""
            , "## 📁 异常小文件（< 1KB）"
            ]
            ++ unlines (map formatSmall small)
            ++ unlines
            [ ""
            , "---"
            , "*此文件由 ebook renamer 自动生成*"
            ]

    let path = case optTodoFile opts of
            Just p -> p
            Nothing -> (optPath opts) </> "todo.md"

    writeFile path content

formatFailed :: FileInfo -> String
formatFailed f = "- [ ] 重新下载: " ++ fiOriginalName f ++ " (未完成下载)"

formatSmall :: FileInfo -> String
formatSmall f = "- [ ] 检查并重新下载: " ++ fiOriginalName f ++ " (文件过小，仅 " ++ show (fiSize f) ++ " 字节)"
