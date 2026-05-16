{-# LANGUAGE OverloadedStrings #-}

module Normalizer
    ( normalizeFile
    ) where

import Scanner (FileInfo(..))
import Data.Text (Text)
import qualified Data.Text as T
import Text.Regex.TDFA ((=~))
import Data.List (sortBy, foldl')
import Data.Maybe (listToMaybe, fromMaybe)
import System.FilePath (takeExtension, dropExtension)

-- Series mappings
seriesMap :: [(Text, Text)]
seriesMap =
    [ ("Graduate Texts in Mathematics", "GTM")
    , ("Cambridge Studies in Advanced Mathematics", "CSAM")
    , ("London Mathematical Society Lecture Note Series", "LMSLN")
    , ("Progress in Mathematics", "PM")
    , ("Springer Undergraduate Mathematics Series", "SUMS")
    , ("Graduate Studies in Mathematics", "GSM")
    , ("AMS Mathematical Surveys and Monographs", "AMS-MSM")
    , ("Oxford Graduate Texts in Mathematics", "OGTM")
    , ("Springer Monographs in Mathematics", "SMM")
    ]

-- Source indicators to remove
sourceIndicators :: [Text]
sourceIndicators =
    [ " - libgen.li"
    , " - libgen"
    , " - Z-Library"
    , " - z-Library"
    , " - Anna's Archive"
    , " (Z-Library)"
    , " (z-Library)"
    , " (libgen.li)"
    , " (libgen)"
    , " (Anna's Archive)"
    , " libgen.li.pdf"
    , " libgen.pdf"
    , " Z-Library.pdf"
    , " z-Library.pdf"
    , " Anna's Archive.pdf"
    ]

-- Main normalization function
normalizeFile :: FileInfo -> FileInfo
normalizeFile info
    | fiIsFailedDownload info || fiIsTooSmall info = info
    | otherwise =
        let originalName = T.pack $ fiOriginalName info
            ext = T.pack $ fiExtension info

            -- Step 1: Remove .download suffix (handled by classification, but cleanup name)
            -- Step 2: Remove extension suffix
            nameNoExt = if T.null ext then originalName else T.dropEnd (T.length ext) originalName

            -- Step 3: Strip whitespace
            cleanName1 = T.strip nameNoExt

            -- Step 4: Extract Series
            (cleanName2, seriesPart) = extractSeries cleanName1

            -- Step 5: Clean source indicators
            cleanName3 = removeSourceIndicators cleanName2

            -- Step 6: Extract Year
            (cleanName4, yearPart) = extractYear cleanName3

            -- Step 7: Edition Detection
            (cleanName5, editionPart) = extractEdition cleanName4

            -- Step 8: Split Author/Title
            (author, title) = splitAuthorTitle cleanName5

            -- Step 9: Final Assembly
            finalName = assembleName author title seriesPart yearPart editionPart ext

        in info { fiNewName = Just (T.unpack finalName) }

extractSeries :: Text -> (Text, Maybe Text)
extractSeries name =
    -- Try to find series patterns
    -- Pattern 1: Series Name Volume - ...
    -- Pattern 2: (Series Name Volume) ...
    -- Simplified for now: just return original name and Nothing if no regex match logic implemented yet properly
    -- Implementing regex in Haskell without compiler feedback is risky for complex patterns.
    -- I will implement a simplified version or try my best with standard patterns.

    -- Let's try to match known series
    let match = findSeriesMatch name
    in case match of
        Just (sName, sAbbr, sVol) ->
            let pattern1 = sName <> " " <> sVol <> " - "
                pattern2 = "(" <> sName <> " " <> sVol <> ")"
            in if pattern1 `T.isInfixOf` name
               then (T.replace pattern1 "" name, Just (sAbbr <> " " <> sVol))
               else if pattern2 `T.isInfixOf` name
                    then (T.replace pattern2 "" name, Just (sAbbr <> " " <> sVol))
                    else (name, Nothing)
        Nothing -> (name, Nothing)

findSeriesMatch :: Text -> Maybe (Text, Text, Text)
findSeriesMatch name =
    -- This would need regex to extract volume
    -- For now, returning Nothing to be safe
    Nothing

removeSourceIndicators :: Text -> Text
removeSourceIndicators name =
    foldl' (\acc ind -> T.replace ind "" acc) name sourceIndicators

extractYear :: Text -> (Text, Maybe Text)
extractYear name =
    let yearRegex = "(19|20)[0-9]{2}" :: Text
        matches = getAllMatches (name =~ yearRegex :: (Int, Int)) :: [(Int, Int)]
        -- Need actual regex library usage here.
        -- `name =~ regex` returns different things based on type context.
        -- Assuming we can find the year.
        -- As a placeholder logic without robust regex testing:

        -- Let's assume we find the last occurrence of 4 digits starting with 19 or 20
        maybeYear = findLastYear name
    in case maybeYear of
        Just year ->
            let clean = removeYearPatterns name year
            in (clean, Just year)
        Nothing -> (name, Nothing)

findLastYear :: Text -> Maybe Text
findLastYear name =
    -- Manual implementation to be safe without regex
    let words' = T.words $ T.map (\c -> if c `elem` ("()[],." :: String) then ' ' else c) name
        years = filter isYear words'
    in if null years then Nothing else Just (last years)

isYear :: Text -> Bool
isYear t = T.length t == 4 && (T.isPrefixOf "19" t || T.isPrefixOf "20" t) && T.all (\c -> c >= '0' && c <= '9') t

removeYearPatterns :: Text -> Text -> Text
removeYearPatterns name year =
    let p1 = "(" <> year <> ")"
        p2 = "(" <> year <> ","
        p3 = ", " <> year <> ")"
        p4 = " " <> year <> " "
    in T.replace p1 "" $ T.replace p2 "(" $ T.replace p3 ")" name
       -- Note: this is a simplification.

extractEdition :: Text -> (Text, Maybe Text)
extractEdition name =
    if "2nd Edition" `T.isInfixOf` name || "2nd ed" `T.isInfixOf` name then (T.replace "2nd Edition" "" $ T.replace "2nd ed" "" name, Just "2nd ed")
    else if "3rd Edition" `T.isInfixOf` name || "3rd ed" `T.isInfixOf` name then (T.replace "3rd Edition" "" $ T.replace "3rd ed" "" name, Just "3rd ed")
    -- Add more cases as needed
    else (name, Nothing)

splitAuthorTitle :: Text -> (Maybe Text, Text)
splitAuthorTitle name =
    if " - " `T.isInfixOf` name
    then
        let parts = T.splitOn " - " name
            -- Heuristic: First part is author if it looks like an author
            possibleAuthor = head parts
            rest = T.intercalate " - " (tail parts)
        in (Just (cleanString possibleAuthor), cleanString rest)
    else (Nothing, cleanString name)

cleanString :: Text -> Text
cleanString = T.strip . T.dropAround (\c -> c `elem` ("-_,.;:[]()" :: String))

assembleName :: Maybe Text -> Text -> Maybe Text -> Maybe Text -> Maybe Text -> Text -> Text
assembleName author title series year edition ext =
    let authorPart = maybe "" (<> " - ") author
        seriesPartStr = maybe "" (\s -> " [" <> s <> "]") series
        yearEdPart = case (year, edition) of
            (Just y, Just e) -> " (" <> y <> ", " <> e <> ")"
            (Just y, Nothing) -> " (" <> y <> ")"
            (Nothing, Just e) -> " (" <> e <> ")"
            (Nothing, Nothing) -> ""
    in authorPart <> title <> seriesPartStr <> yearEdPart <> ext

-- Helper for regex (placeholder as we rely on manual parsing for safety in this environment)
getAllMatches :: (Int, Int) -> [(Int, Int)]
getAllMatches _ = []
