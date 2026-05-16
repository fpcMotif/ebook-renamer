module Options
    ( Options(..)
    , parseOptions
    ) where

import Options.Applicative

data Options = Options
    { optPath            :: FilePath
    , optDryRun          :: Bool
    , optMaxDepth        :: Int
    , optNoRecursive     :: Bool
    , optExtensions      :: String
    , optNoDelete        :: Bool
    , optTodoFile        :: Maybe FilePath
    , optLogFile         :: Maybe FilePath
    , optPreserveUnicode :: Bool
    , optFetchArxiv      :: Bool
    , optVerbose         :: Bool
    , optDeleteSmall     :: Bool
    , optJson            :: Bool
    , optSkipCloudHash   :: Bool
    } deriving (Show, Eq)

optionsParser :: Parser Options
optionsParser = Options
    <$> strArgument
        ( metavar "PATH"
       <> help "Target directory to scan and rename (defaults to current directory)"
       <> value "."
       <> showDefault )
    <*> switch
        ( long "dry-run"
       <> short 'd'
       <> help "Show changes without applying them" )
    <*> option auto
        ( long "max-depth"
       <> metavar "DEPTH"
       <> help "Maximum directory depth to traverse"
       <> value maxBound
       <> showDefault )
    <*> switch
        ( long "no-recursive"
       <> help "Sets effective max-depth to 1 (top-level only)" )
    <*> strOption
        ( long "extensions"
       <> metavar "EXT1,EXT2"
       <> help "Comma-separated extensions to process"
       <> value "pdf,epub,txt"
       <> showDefault )
    <*> switch
        ( long "no-delete"
       <> help "Don't delete duplicate files, only list them" )
    <*> optional (strOption
        ( long "todo-file"
       <> metavar "PATH"
       <> help "Path to write todo.md file" ))
    <*> optional (strOption
        ( long "log-file"
       <> metavar "PATH"
       <> help "Optional path to write detailed operation log" ))
    <*> switch
        ( long "preserve-unicode"
       <> help "Preserve original non-Latin script (unused)" )
    <*> switch
        ( long "fetch-arxiv"
       <> help "Fetch arXiv metadata via API (placeholder)" )
    <*> switch
        ( long "verbose"
       <> short 'v'
       <> help "Enable verbose logging" )
    <*> switch
        ( long "delete-small"
       <> help "Delete small/corrupted files (< 1KB) instead of adding to todo list" )
    <*> switch
        ( long "json"
       <> help "Output operations in JSON format" )
    <*> switch
        ( long "skip-cloud-hash"
       <> help "Skip MD5 hash computation for duplicate detection" )

parseOptions :: IO Options
parseOptions = execParser opts
  where
    opts = info (optionsParser <**> helper)
      ( fullDesc
     <> progDesc "Batch rename and organize downloaded books and arXiv files"
     <> header "ebook-renamer - Organize your ebook collection" )
